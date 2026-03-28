# Architecture Research

**Domain:** Rust CLI disk scanning and cleanup utility (macOS)
**Researched:** 2026-03-28
**Confidence:** HIGH

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        CLI Layer                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  cli.rs — clap derive: Cli struct + Commands enum         │   │
│  │  commands/ — one file per subcommand (summary, scan, ...) │   │
│  └───────────────────────────┬──────────────────────────────┘   │
└───────────────────────────────┼─────────────────────────────────┘
                                │ calls into
┌───────────────────────────────▼─────────────────────────────────┐
│                      Core Pipeline Layer                         │
│                                                                  │
│   fs_scan ──► classify ──► analyze ──► cleanup                  │
│       │           │            │           │                     │
│  (DirEntry)  (FileEntry)  (ScanResult) (CleanupCandidate)        │
│                                                                  │
│  Data flows left-to-right. Later stages cannot call earlier.     │
└───────────────────┬─────────────────────────────────────────────┘
                    │ reads from
┌───────────────────▼─────────────────────────────────────────────┐
│                     Support Layer                                │
│  ┌──────────┐  ┌────────────────┐  ┌──────────────────────┐     │
│  │  config  │  │ platform::macos │  │       output         │     │
│  │ (TOML)   │  │ (statvfs, trash)│  │ (tables, JSON/stdout)│     │
│  └──────────┘  └────────────────┘  └──────────────────────┘     │
└─────────────────────────────────────────────────────────────────┘
```

The pipeline is strictly one-directional. `cleanup` depends on `classify` having run; it cannot be invoked before scan and classification produce reliable data. This enforces the Inspect → Classify → Preview → Clean safety sequence at the architectural level, not just through runtime guards.

### Component Responsibilities

| Component | Responsibility | What it owns |
|-----------|----------------|--------------|
| `cli` | Argument parsing, subcommand routing | `Cli` struct, `Commands` enum, global flags (`--json`, `--path`) |
| `commands/` | One file per subcommand; bridges CLI to core | Calls `fs_scan`, `analyze`, `classify`, `cleanup`; feeds `output` |
| `fs_scan` | Raw directory traversal, metadata collection | `DirEntry` stream (via `walkdir`), symlink handling, error tolerance |
| `classify` | Assigns category and safety class to each entry | Classification rules: path-first, then known macOS dirs, then extension |
| `analyze` | Aggregation, top-N ranking, category totals | `ScanResult`, `CategoryTotal`, sorted `largest_files` / `largest_dirs` |
| `cleanup` | Preview generation and deletion execution | `CleanupCandidate` list, Trash call, protected-path enforcement, log write |
| `config` | Loads and validates `~/.config/Freespace/config.toml` | `Config` struct, exclusion lists, safe_categories |
| `output` | Renders to human-readable tables or clean JSON | Writes JSON to stdout; writes logs/errors to stderr |
| `platform::macos` | macOS-specific syscalls | `statvfs` for volume info, `trash` crate delegation, macOS known-path map |

## Recommended Project Structure

```
src/
├── main.rs                  # binary entry point: parse CLI, load config, dispatch
├── cli.rs                   # Cli struct and Commands enum (clap derive)
├── commands/
│   ├── mod.rs               # re-exports all command handlers
│   ├── summary.rs           # freespace summary
│   ├── scan.rs              # freespace scan <path>
│   ├── categories.rs        # freespace categories <path>
│   ├── hidden.rs            # freespace hidden <path>
│   ├── caches.rs            # freespace caches
│   ├── clean.rs             # freespace clean preview / apply
│   └── doctor.rs            # freespace doctor
├── fs_scan/
│   ├── mod.rs               # public API: scan_path() -> impl Iterator<Item=DirEntry>
│   └── walker.rs            # walkdir wrapper, error tolerance, symlink policy
├── classify/
│   ├── mod.rs               # public API: classify(path, config) -> FileEntry
│   ├── path_rules.rs        # macOS known-path table (~/.ollama, ~/Library/Caches, …)
│   ├── extension_rules.rs   # extension → category fallback table
│   └── safety.rs            # safety class assignment (safe/caution/dangerous/blocked)
├── analyze/
│   ├── mod.rs               # public API: aggregate(iter) -> ScanResult
│   └── top_n.rs             # bounded heap for largest_files / largest_dirs
├── cleanup/
│   ├── mod.rs               # public API: preview(), apply()
│   ├── preview.rs           # builds CleanupCandidate list without side effects
│   ├── executor.rs          # Trash call, --force permanent delete, protected-path guard
│   └── log.rs               # appends to ~/.local/state/Freespace/cleanup.log
├── config/
│   ├── mod.rs               # load_config() -> Config
│   └── schema.rs            # Config struct (serde + toml)
├── output/
│   ├── mod.rs               # Renderer trait: render_table() / render_json()
│   ├── table.rs             # comfy-table formatting
│   └── json.rs              # serde_json to stdout; errors to stderr
└── platform/
    └── macos.rs             # volume_info() via statvfs, known macOS path map, OS version
```

### Structure Rationale

- **`commands/`**: Each file corresponds to one CLI subcommand. Commands are thin — they call into core modules, then call `output`. No business logic lives here.
- **`fs_scan/`**: Isolated so it can be replaced or mocked for integration tests. Returns an iterator, never a `Vec`.
- **`classify/`**: Split into `path_rules`, `extension_rules`, and `safety` to make each table independently editable and independently testable.
- **`analyze/`**: Pure functions over iterators. No I/O. Easiest module to unit-test.
- **`cleanup/`**: Preview is separated from execution so `clean preview` can call `preview.rs` without touching `executor.rs`. This is an architectural enforcement of the safety order.
- **`platform/`**: All macOS-specific code lives here. If Linux support is added later, a new `platform/linux.rs` with the same public API is all that is needed.

## Architectural Patterns

### Pattern 1: Iterator-based streaming scan (no buffering)

**What:** `fs_scan::scan_path()` returns `impl Iterator<Item = Result<DirEntry, ScanError>>`. Callers consume with `.fold()` or `.for_each()`. Nothing is collected into a `Vec` until a top-N structure requires it.

**When to use:** Always for traversal. This is the only approach that handles 100k+ file trees without OOM risk.

**Trade-offs:** Slightly harder to compose than returning a `Vec`, but Rust's iterator combinators make it ergonomic. Parallel traversal with `jwalk` or `rayon` is an opt-in upgrade path.

**Example:**
```rust
// fs_scan/mod.rs
pub fn scan_path(root: &Path, config: &Config) -> impl Iterator<Item = Result<FileEntry, ScanError>> {
    WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !config.is_excluded(e.path()))
        .filter_map(|res| match res {
            Ok(entry) => Some(Ok(into_file_entry(entry))),
            Err(e) if is_permission_error(&e) => None, // skip, log to stderr
            Err(e) => Some(Err(ScanError::from(e))),
        })
}
```

### Pattern 2: Classify-then-aggregate pipeline

**What:** `classify::classify()` is applied per entry as the iterator is consumed by `analyze::aggregate()`. Classification is a pure function: `(path, config) -> FileEntry`. Aggregation folds classified entries into a `ScanResult`.

**When to use:** Every command that groups by category or shows safety classes.

**Trade-offs:** Single-pass over the directory tree. Classification happens inline, not as a separate pass. This keeps memory bounded and avoids storing the full entry list.

**Example:**
```rust
// commands/categories.rs
pub fn run(path: &Path, config: &Config) -> ScanResult {
    let entries = fs_scan::scan_path(path, config)
        .filter_map(|r| r.ok())
        .map(|entry| classify::classify(entry, config));
    analyze::aggregate(entries)
}
```

### Pattern 3: Preview/Execute split for cleanup safety

**What:** `cleanup::preview()` returns `Vec<CleanupCandidate>` — a pure read operation. `cleanup::apply()` takes a `Vec<CleanupCandidate>` and executes. The CLI enforces that `preview` runs first via the `clean preview` / `clean apply` command design; `apply` requires the user to have seen the preview output.

**When to use:** All cleanup operations. This is non-negotiable given the safety model.

**Trade-offs:** Requires two user-facing commands rather than one. This is intentional and a feature, not a limitation.

### Pattern 4: Stdout/Stderr separation for JSON scriptability

**What:** `output::json` writes exclusively to stdout. All progress indicators, warnings, and errors go to stderr via `eprintln!` or `tracing`. This means `freespace scan --json | jq` works without contamination.

**When to use:** Every command, via the `output` module. Never call `println!` directly from command handlers.

**Trade-offs:** Requires discipline in the codebase. The `output` module as a single rendering point enforces this.

## Data Flow

### Full pipeline — `freespace categories <path>`

```
User: freespace categories ~/Downloads
          │
          ▼
cli.rs: parse → Commands::Categories { path }
          │
          ▼
commands/categories.rs
          │
          ├─► config::load_config()        ← reads ~/.config/Freespace/config.toml
          │
          ├─► fs_scan::scan_path(path)     ← walkdir iterator, no buffering
          │         │
          │    DirEntry stream
          │         │
          ├─► classify::classify(entry)    ← path rules → macOS known dirs → extension fallback
          │         │
          │    FileEntry (with category + safety)
          │         │
          ├─► analyze::aggregate(entries)  ← fold into ScanResult
          │         │
          │    ScanResult { category_totals, file_count, total_bytes }
          │
          └─► output::render(result, --json flag)
                    │
              stdout: table or JSON
              stderr: any permission warnings
```

### Cleanup pipeline — `freespace clean preview` then `freespace clean apply`

```
Step 1: freespace clean preview
          │
          ▼
cleanup::preview(config)
  └─► classify::safety(path, config)  → CleanupCandidate list
  └─► output::render_preview()        → stdout (table or JSON, no side effects)

Step 2: user reviews output

Step 3: freespace clean apply
          │
          ▼
cleanup::apply(candidates, --force flag)
  └─► protected_path_guard()          → hard block on /System, /usr, /bin, /sbin, /private
  └─► trash::delete(path)             → Trash by default
      OR fs::remove_file(path)        → only if --force passed
  └─► cleanup::log::append()          → ~/.local/state/Freespace/cleanup.log
```

### Config and platform injection

```
main.rs
  └─► config::load_config()     → Config (passed by reference to all commands)
  └─► platform::macos::check()  → validates macOS environment (freespace doctor)

platform::macos
  └─► volume_info()             → calls statvfs/libc, returns VolumeInfo
  └─► KNOWN_PATHS map           → static table used by classify::path_rules
```

## Build Order

The module dependency graph dictates build order. Lower numbers have no upstream dependencies.

| Order | Module | Depends On | Why build here |
|-------|--------|-----------|----------------|
| 1 | `config` | nothing | All other modules accept `&Config`; must exist first |
| 2 | `platform::macos` | `config` | Provides `VolumeInfo` and known-path table used by `classify` |
| 3 | `fs_scan` | `config` | Traversal iterator; needed by all scan commands |
| 4 | `classify` | `config`, `platform::macos` | Needs known-path table; produces `FileEntry` |
| 5 | `analyze` | `classify` | Consumes `FileEntry` stream; produces `ScanResult` |
| 6 | `output` | `analyze`, `classify` | Renders all result types; can be built once types exist |
| 7 | `cleanup` | `classify`, `config`, `platform::macos` | Safety-critical: only built after classify is reliable |
| 8 | `cli` + `commands/` | all above | Wire-up layer: all modules must exist before commands call them |

**Critical build gate:** `cleanup` (order 7) must not be built until `classify` (order 4) has complete and tested classification rules. This maps directly to the PRD instruction: "Never implement cleanup before scan and classification are reliable."

## Safety Pipeline Mapping

```
Inspect        →    Classify       →    Preview        →    Clean
──────────────────────────────────────────────────────────────────
fs_scan            classify            cleanup::           cleanup::
(DirEntry          (FileEntry          preview()           apply()
 stream)            + safety)          (read-only,         (side effects,
                                        no writes)          logged)

freespace scan     freespace           freespace clean     freespace clean
freespace hidden   categories          preview             apply
freespace caches   freespace hidden
                   freespace caches
```

The four stages map to four separate code paths. There is no shortcut from `fs_scan` directly to `cleanup::apply`. Every cleanup operation must pass through `classify::safety()` first.

## Performance Considerations

### Streaming traversal (100k+ files)

`walkdir` is iterator-based and makes near-minimal syscalls. It stores only the current directory handle and unyielded entries for that level — memory scales with tree depth, not file count. For 100k files in a flat directory, peak allocation is a single directory's entry list, not 100k entries.

**Concrete guidance:**
- Never `.collect()` the `scan_path()` iterator into a `Vec<FileEntry>` in command handlers.
- `analyze::aggregate()` must use `.fold()` with a bounded accumulator.
- Top-N lists (largest files) use a `BinaryHeap` capped at N, not a full sort of all entries.

### Top-N using bounded heap

```rust
// analyze/top_n.rs
pub fn top_n_files(iter: impl Iterator<Item = FileEntry>, n: usize) -> Vec<FileEntry> {
    let mut heap = BinaryHeap::with_capacity(n + 1);
    for entry in iter {
        heap.push(Reverse(entry.size));
        if heap.len() > n {
            heap.pop(); // evict smallest, keeping top N
        }
    }
    heap.into_sorted_vec()
}
```

### Error tolerance during traversal

`walkdir` yields `Result<DirEntry>` for each entry. Permission errors, broken symlinks, and mid-scan deletions must be handled at the iterator level in `fs_scan`, not propagated to command handlers. The pattern: log to stderr and continue — never abort the entire scan.

### Parallel traversal (future consideration)

`jwalk` is a drop-in parallel alternative to `walkdir` that streams results in sorted order using rayon. It maximises SSD throughput. For v1, `walkdir` is sufficient. If scan latency on large directories is flagged post-launch, `jwalk` is the upgrade path with minimal refactoring cost because `fs_scan` is the only consumer of traversal.

## Anti-Patterns

### Anti-Pattern 1: Buffering the full directory tree

**What people do:** `let all_entries: Vec<_> = scan_path(root).collect();`
**Why it's wrong:** On a 500k-file directory, this allocates multiple hundred MB. The tool hangs before showing any output.
**Do this instead:** Process entries as a stream via `fold()` or `for_each()`. Only materialise bounded structures (top-N heaps, category totals map).

### Anti-Pattern 2: Calling cleanup from inside scan/classify

**What people do:** Add a `--cleanup` flag to `freespace scan` that deletes matched files during traversal.
**Why it's wrong:** Violates the safety model. The user never sees what will be deleted. No preview, no protected-path check, no log entry.
**Do this instead:** `freespace clean preview` then `freespace clean apply` as separate commands. The architecture enforces this via the module boundary.

### Anti-Pattern 3: Writing JSON to stderr (or logs to stdout)

**What people do:** Mix `println!` and `eprintln!` across command handlers ad hoc.
**Why it's wrong:** `freespace scan --json | jq` breaks silently if log output contaminates stdout.
**Do this instead:** Route all rendering through `output::render()`. JSON → stdout. Everything else → stderr. Enforce at the module level.

### Anti-Pattern 4: Hardcoding macOS paths outside `platform::macos`

**What people do:** Scatter `if path.starts_with("/System")` checks across `classify`, `cleanup`, `commands/`.
**Why it's wrong:** macOS path behaviour changes across OS versions. Centralised ownership means one change location.
**Do this instead:** All macOS-specific path logic lives in `platform::macos::KNOWN_PATHS` and `platform::macos::PROTECTED_PATHS`. `classify` and `cleanup` read from that module.

### Anti-Pattern 5: Extension-first classification

**What people do:** Check file extension first, then fall back to path.
**Why it's wrong:** A `.db` file in `~/Library/Caches/` is a cache, not a database document. Path context is more reliable than extension on macOS.
**Do this instead:** Path rules first → known macOS dirs second → extension mapping third → fallback unknown. This is the defined classification priority order.

## Integration Points

### Internal Module Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `cli` → `commands/` | Direct function call | Commands are plain functions, not trait objects |
| `commands/` → `fs_scan` | Returns `impl Iterator` | Commands never hold the whole tree in memory |
| `commands/` → `classify` | Per-entry function call | `classify()` is a pure function, easily tested |
| `commands/` → `analyze` | `aggregate(iter)` call | Takes the classified iterator |
| `commands/` → `cleanup` | `preview()` / `apply()` calls | Separated; `apply` requires explicit user command |
| any module → `config` | `&Config` reference | Passed from `main.rs`; not loaded more than once |
| any module → `platform::macos` | Module-level constants + functions | `KNOWN_PATHS`, `PROTECTED_PATHS`, `volume_info()` |
| `commands/` → `output` | `render(result, format)` | Only exit point for stdout |

### External Integrations

| Dependency | Integration | Notes |
|------------|-------------|-------|
| `walkdir 2.x` | Called in `fs_scan/walker.rs` | Battle-tested (291M+ downloads), iterator-native |
| `clap 4.x` (derive) | Used in `cli.rs` | Derive API preferred for compile-time validation |
| `serde` + `toml` | Used in `config/schema.rs` | Config deserialization |
| `serde_json` | Used in `output/json.rs` | JSON output to stdout |
| `trash` | Called in `cleanup/executor.rs` | macOS Trash; permanent delete via `std::fs::remove_file` under `--force` |
| `comfy-table` | Used in `output/table.rs` | Human-readable table rendering |
| `libc` / `nix` | Used in `platform/macos.rs` | `statvfs` for volume info |

## Sources

- [walkdir crate docs — docs.rs](https://docs.rs/walkdir/)
- [walkdir GitHub — BurntSushi/walkdir](https://github.com/BurntSushi/walkdir)
- [Rust CLI recommendations — argument handling](https://rust-cli-recommendations.sunshowers.io/handling-arguments.html)
- [Kevin K. — CLI Structure in Rust](https://kbknapp.dev/cli-structure-01/)
- [dua-cli GitHub — Byron/dua-cli](https://github.com/Byron/dua-cli) (reference implementation for streaming disk aggregation)
- [Rust Iterator docs — fold, scan](https://doc.rust-lang.org/std/iter/trait.Iterator.html)
- [sysinfo crate — lib.rs](https://lib.rs/crates/sysinfo) (alternative to libc statvfs for disk info)

---
*Architecture research for: Rust CLI disk scanning and cleanup utility (macOS)*
*Researched: 2026-03-28*
