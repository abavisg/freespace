# Phase 3: Core Scan Engine - Research

**Researched:** 2026-03-29
**Domain:** Rust filesystem traversal, physical disk size accounting, hardlink deduplication, APFS/macOS-specific metadata
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

All implementation choices are at Claude's discretion. Key constraints from research and PITFALLS.md:

**Safety-critical (from research):**
- Use `walkdir` 2.5 for streaming traversal — iterator-based, memory proportional to tree depth not file count
- Hardlink deduplication via `(dev, ino)` HashSet — use `std::os::unix::fs::MetadataExt` for ino() and dev()
- Physical size via `metadata.blocks() * 512` (NOT `metadata.len()`) for sparse file correctness
- TCC/permission denied errors: count skipped paths, surface in output, do NOT crash
- Broken symlinks and mid-scan deletions: log to stderr, continue scan, do NOT crash
- `walkdir::WalkDir::new(path).follow_links(false)` — never follow symlinks (prevents loops)

**Architecture:**
- `fs_scan` module: `src/fs_scan/mod.rs` — streaming iterator, no loading full tree into memory
- `analyze` module: `src/analyze/mod.rs` — top-N aggregation using BinaryHeap (added in Phase 5, stub only here)
- `commands/scan.rs` — thin dispatch layer, already exists as stub

**Output:**
- Human-readable table by default (comfy-table)
- `--json` produces clean JSON on stdout
- Skipped paths count shown in output

### Claude's Discretion

All implementation choices are at Claude's discretion subject to the safety-critical constraints above.

### Deferred Ideas (OUT OF SCOPE)

- jwalk parallel traversal (deferred to post-MVP optimization)
- Progress bar for long-running scans (v2)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SCAN-01 | `freespace scan <path>` reports total size, file count, directory count, largest files (top-N), and largest directories (top-N) | `ScanResult` struct + comfy-table output pattern established; top-N via BinaryHeap stub in analyze module |
| SCAN-02 | Scanner uses streaming walkdir traversal — no loading full directory trees into memory | `WalkDir::new().follow_links(false).into_iter()` is iterator-based; never `.collect()` into Vec; memory scales with depth not file count |
| SCAN-03 | Scanner deduplicates hardlinks via `(dev, ino)` tracking to prevent double-counting | `HashSet<(u64, u64)>` + `MetadataExt::dev()` and `MetadataExt::ino()` — exact pattern documented in PITFALLS.md |
| SCAN-04 | Scanner uses physical size (`st_blocks * 512`) for sparse files, not logical `metadata().len()` | `MetadataExt::blocks()` returns 512-byte units; `blocks() * 512` = physical bytes on APFS |
| SCAN-05 | Scanner handles permission errors, broken symlinks, and files deleted during scan without crashing — skipped paths are counted and surfaced | `walkdir::Error::io_error().kind() == PermissionDenied` pattern; `loop_ancestor()` for symlink loops; match-based error handling per entry |
</phase_requirements>

---

## Summary

Phase 3 implements the core scan engine — the foundation every downstream phase depends on. The primary challenge is correctness, not performance: hardlink deduplication, physical block-based sizing, and resilient error handling must all be correct from the first commit because later phases (categories, cleanup) inherit whatever `fs_scan` produces.

The stack is already decided and fully present in Cargo.toml: `walkdir 2.5` for traversal, `std::os::unix::fs::MetadataExt` for physical metadata, `bytesize` for human-readable formatting, and `comfy-table 7.2` for table output. The `output::write_json()` helper already exists. The `commands/scan.rs` stub already has the correct function signature `run(path, config, json) -> anyhow::Result<()>`.

The module to create is `src/fs_scan/mod.rs` (streaming iterator returning `ScanResult`). The module to stub is `src/analyze/mod.rs` (will be filled in Phase 5; Phase 3 needs only enough to support the scan output). The CONTEXT.md decision is firm: no full-tree buffering, physical sizes everywhere, (dev, ino) deduplication before any byte is added to totals.

**Primary recommendation:** Build `fs_scan::scan_path()` as a streaming fold that accumulates a `ScanResult` directly — file count, dir count, total physical bytes, skip count, and a bounded top-N heap for files — without ever collecting all entries into a Vec.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| walkdir | 2.5 | Streaming directory traversal | Iterator-native, battle-tested (291M+ downloads), memory scales with depth not file count; already in Cargo.toml |
| std::os::unix::fs::MetadataExt | stdlib | Physical block count, inode, device ID | The only stable way to access st_blocks/ino/dev on Unix from safe Rust |
| std::collections::HashSet | stdlib | Hardlink deduplication via (dev, ino) pairs | Zero-dependency, correct, O(1) insert/lookup |
| std::collections::BinaryHeap | stdlib | Bounded top-N for largest files/dirs | O(n log N) vs O(n log n) for full sort; bounded memory |
| bytesize | 2.3 | Human-readable byte formatting | Already in Cargo.toml; used in summary command |
| comfy-table | 7.2 | Terminal table output | Already in Cargo.toml; used in summary command; established pattern |
| serde + serde_json | 1.0 | JSON output via write_json() | Already in Cargo.toml; write_json() helper already exists in output module |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| anyhow | 1.0 | Error propagation in command handler | Already used in all command handlers |
| tracing | 0.1 | Stderr warning/debug logging during traversal | Already wired in main.rs; use warn!/debug! for skipped paths |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| walkdir | jwalk | jwalk is parallel (rayon-based) but adds complexity; deferred to v2 per CONTEXT.md |
| HashSet (SipHash) | AHashSet | AHashSet is faster for high-inode counts (1M+) but not needed for MVP |
| stdlib BinaryHeap | indexmap or sorted_vec | No advantage; BinaryHeap with Reverse<> is idiomatic and zero-dep |

**Installation:** All packages already in Cargo.toml. No new dependencies required for this phase.

---

## Architecture Patterns

### Recommended Module Structure for Phase 3

```
src/
├── fs_scan/
│   └── mod.rs          # scan_path() -> ScanResult; FileEntry type; error handling
├── analyze/
│   └── mod.rs          # STUB ONLY: ScanResult type lives here; top-N filled in Phase 5
├── commands/
│   └── scan.rs         # run() — calls fs_scan::scan_path(), calls output
└── output/
    └── mod.rs          # write_json() already exists; add render_scan_table()
```

### Key Types to Define

```
ScanResult (in src/analyze/mod.rs — stub):
  total_bytes: u64          // physical bytes (blocks * 512), deduplicated
  file_count: u64
  dir_count: u64
  skipped_count: u64        // permission denied + other non-fatal errors
  largest_files: Vec<FileEntry>  // top-N by physical size

FileEntry (in src/fs_scan/mod.rs):
  path: PathBuf
  size: u64                 // physical bytes (blocks * 512)
  is_dir: bool
```

### Pattern 1: Streaming Traversal with Inline Error Handling

**What:** `fs_scan::scan_path()` iterates with `WalkDir` and handles each `Result<DirEntry>` inline via `match`. Permission denied increments the skip counter. Loop errors log to stderr and continue. Missing file (ENOENT) during traversal logs and continues. Only genuinely unexpected errors surface to the caller.

**When to use:** Always. This is the only correct pattern for resilient scanning.

```rust
// src/fs_scan/mod.rs
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;
use walkdir::WalkDir;

pub fn scan_path(root: &std::path::Path, config: &crate::config::schema::Config) -> ScanResult {
    let mut result = ScanResult::default();
    let mut seen_inodes: HashSet<(u64, u64)> = HashSet::new();

    for entry_result in WalkDir::new(root).follow_links(false) {
        match entry_result {
            Ok(entry) => {
                // check config exclusions
                if config.scan.exclude.iter().any(|ex| entry.path().starts_with(ex)) {
                    continue;
                }
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("metadata error at {:?}: {}", entry.path(), e);
                        result.skipped_count += 1;
                        continue;
                    }
                };
                if metadata.is_dir() {
                    result.dir_count += 1;
                } else if metadata.is_file() {
                    // Hardlink deduplication — count each inode only once
                    let key = (metadata.dev(), metadata.ino());
                    if seen_inodes.insert(key) {
                        let physical = metadata.blocks() * 512;
                        result.total_bytes += physical;
                        result.file_count += 1;
                        // top-N heap update goes here (Phase 5 fills this)
                    }
                }
                // symlinks: entry.path_is_symlink() == true when follow_links(false)
                // symlinks are NOT counted in file_count or total_bytes
            }
            Err(e) => {
                // Check for symlink loop
                if e.loop_ancestor().is_some() {
                    tracing::warn!("symlink loop detected at {:?}", e.path());
                    result.skipped_count += 1;
                    continue;
                }
                // Check for permission denied
                if let Some(io_err) = e.io_error() {
                    if io_err.kind() == std::io::ErrorKind::PermissionDenied {
                        tracing::warn!("permission denied at {:?}", e.path());
                        result.skipped_count += 1;
                        continue;
                    }
                }
                // All other errors (ENOENT mid-scan, broken symlink stat): log and continue
                tracing::warn!("scan error at {:?}: {}", e.path(), e);
                result.skipped_count += 1;
            }
        }
    }
    result
}
```

**Source:** walkdir 2.5 docs — `Error::loop_ancestor()`, `Error::io_error()`, `Error::path()` confirmed methods. The `filter_map(|e| e.ok())` shortcut silently drops ALL errors including permission denied — never use it when skip counting is required.

### Pattern 2: Physical Size Calculation

**What:** `metadata.blocks() * 512` gives the number of bytes actually allocated on disk (st_blocks in 512-byte units). This is the POSIX physical size, not the logical file length.

**When to use:** For every file entry. Never use `metadata.len()` as the primary size metric.

```rust
// Source: std::os::unix::fs::MetadataExt — verified from Rust stdlib docs
use std::os::unix::fs::MetadataExt;

let physical_bytes = metadata.blocks() * 512;  // blocks() returns u64
let logical_bytes = metadata.len();             // use only for display "file size" if needed
```

**Note on APFS sparse files:** On APFS, a 100 GB sparse `.vmdk` may have only 12 GB allocated. `blocks() * 512` returns 12 GB (correct); `len()` returns 100 GB (incorrect for disk-usage purposes).

### Pattern 3: Hardlink Deduplication

**What:** A `HashSet<(u64, u64)>` of `(dev, ino)` pairs. Insert returns `true` if the pair is new; only count bytes on first insert. Both `dev` and `ino` are required — inode numbers are only unique per device.

```rust
// Source: PITFALLS.md + std::os::unix::fs::MetadataExt verified
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;

let mut seen: HashSet<(u64, u64)> = HashSet::new();

// Inside loop, after confirming metadata.is_file():
let key = (metadata.dev(), metadata.ino()); // dev() and ino() confirmed method names
if seen.insert(key) {
    total_bytes += metadata.blocks() * 512;
    file_count += 1;
}
```

### Pattern 4: comfy-table Output (established from summary command)

The existing `summary.rs` command demonstrates the established table pattern:

```rust
// Source: src/commands/summary.rs — existing working code
use comfy_table::Table;
use bytesize::ByteSize;

let mut table = Table::new();
table.set_header(vec!["Path", "Size", "Files", "Dirs", "Skipped"]);
table.add_row(vec![
    root.display().to_string(),
    ByteSize::b(result.total_bytes).to_string(),
    result.file_count.to_string(),
    result.dir_count.to_string(),
    result.skipped_count.to_string(),
]);
println!("{table}");
```

### Pattern 5: Stub for analyze module (Phase 5 fills in)

Phase 5 implements top-N largest files. Phase 3 must define the `ScanResult` type that Phase 5 will extend. The correct place is `src/analyze/mod.rs`:

```rust
// src/analyze/mod.rs — Phase 3 stub
use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct ScanResult {
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub skipped_count: u64,
    // Phase 5 adds: pub largest_files: Vec<crate::fs_scan::FileEntry>
}
```

### Anti-Patterns to Avoid

- **Collecting to Vec:** `scan_path(...).collect::<Vec<_>>()` — allocates O(file_count) memory. Use fold/accumulate directly.
- **`filter_map(|e| e.ok())`:** Silently swallows permission denied errors without incrementing skip counter.
- **`metadata().len()` as size:** Returns logical bytes, not physical allocation. Wrong for sparse files.
- **Only `ino` for dedup:** Inode numbers are device-scoped. Must track `(dev, ino)` pairs.
- **`follow_links(true)`:** Risks infinite loops on symlink cycles. The loop detection exists but requires explicit error handling.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Directory traversal | Custom recursive walk with fs::read_dir | walkdir 2.5 | Loop detection, depth limits, DirEntry caching, 8 years of edge-case fixes |
| Human-readable byte sizes | Custom format_bytes() function | bytesize::ByteSize | Already in Cargo.toml; handles GiB/MiB/KiB correctly |
| Terminal table formatting | Manual column-padding with format!() | comfy-table 7.2 | Already in Cargo.toml; established pattern in summary.rs |
| JSON serialization | Manual JSON string building | serde_json + write_json() | write_json() already exists in output module |

**Key insight:** All needed libraries are already in Cargo.toml from Phase 1. This phase is 100% implementation, zero new dependencies.

---

## Common Pitfalls

### Pitfall 1: Using `filter_map(|e| e.ok())` and Losing Skip Counts

**What goes wrong:** The common `WalkDir::new(path).into_iter().filter_map(|e| e.ok())` idiom silently drops all errors. Permission denied paths are skipped without incrementing the skip counter. The user sees a lower total with no indication of what was missed.

**Why it happens:** It is the simplest error-handling pattern in walkdir's own examples.

**How to avoid:** Use `match` on each entry result. Explicitly handle `PermissionDenied`, `loop_ancestor()`, and all other errors with their own log messages and skip-counter increments.

**Warning signs:** `skipped_count` stays zero even when scanning protected directories.

### Pitfall 2: Physical Size: `blocks()` vs `len()`

**What goes wrong:** `metadata.len()` is used. Docker volumes, sparse VM images, and sparse database files report inflated sizes — sometimes 10x their actual disk use.

**Why it happens:** `len()` is the obvious method; `blocks()` requires the `MetadataExt` trait import.

**How to avoid:** Import `std::os::unix::fs::MetadataExt` at the top of `fs_scan/mod.rs`. Use `metadata.blocks() * 512` everywhere. Never use `metadata.len()` for the `size` field of `FileEntry`.

**Warning signs:** Scan of `~/.vagrant.d` or `~/Library/Containers/com.docker.docker` shows implausibly large totals compared to `du -sh`.

### Pitfall 3: Hardlink Double-Counting

**What goes wrong:** Directories with hardlinks (Git object stores, Docker layers, Time Machine) report 2x–10x inflated totals.

**Why it happens:** walkdir emits one `DirEntry` per path, not per inode. Without `(dev, ino)` tracking, hardlinked files are counted once per path.

**How to avoid:** `HashSet<(u64, u64)>` initialized before the traversal loop. Check `seen.insert(key)` before adding any bytes.

**Warning signs:** Scan total significantly exceeds `du -sh` for the same directory.

### Pitfall 4: Counting Symlinks as Files

**What goes wrong:** `metadata.is_file()` returns false for symlinks when `follow_links(false)`. However, `entry.file_type().is_symlink()` can be used to identify and explicitly skip/count symlinks. If the code calls `entry.metadata()` (which follows symlinks) instead of `entry.path().symlink_metadata()`, it can accidentally stat the symlink target.

**Why it happens:** `DirEntry::metadata()` in walkdir follows symlinks even when `follow_links(false)` is set — it calls `fs::metadata()` not `fs::symlink_metadata()`.

**How to avoid:** For the size calculation, use `entry.metadata()` (which gives target metadata when symlink target exists, or errors when broken). For detecting broken symlinks, `entry.metadata()` will return an error — handle it in the error branch. Symlinks should NOT be added to file_count or have their size counted (to avoid double-counting the target's blocks, which the target entry itself will count).

### Pitfall 5: TCC Permission Denied Looks Like Regular EPERM

**What goes wrong:** TCC denials return `EPERM` (PermissionDenied), identical to Unix permission errors. Both must be handled the same way: log, increment skip counter, continue. The key is that the scanner never crashes — both TCC and regular EPERM are non-fatal.

**Why it happens:** macOS TCC is a second access-control layer invisible to standard Unix APIs. `sudo` does NOT bypass TCC.

**How to avoid:** Treat all `PermissionDenied` errors identically — log path to stderr, increment `skipped_count`, continue traversal.

---

## Code Examples

### Complete FileEntry and ScanResult Types

```rust
// src/fs_scan/mod.rs
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,   // physical bytes: metadata.blocks() * 512
    pub is_dir: bool,
}
```

```rust
// src/analyze/mod.rs (stub — extended in Phase 5)
use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct ScanResult {
    pub root: std::path::PathBuf,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub skipped_count: u64,
}
```

### Import Required for MetadataExt

```rust
// Required import — without this, .blocks(), .ino(), .dev() are not accessible
use std::os::unix::fs::MetadataExt;
```

### Error Handling in walkdir Loop (complete pattern)

```rust
// Source: walkdir::Error documented methods: loop_ancestor(), io_error(), path()
for entry_result in WalkDir::new(root).follow_links(false) {
    match entry_result {
        Ok(entry) => { /* process */ }
        Err(e) => {
            if e.loop_ancestor().is_some() {
                tracing::warn!("symlink loop at {:?}", e.path());
            } else if let Some(io_err) = e.io_error() {
                match io_err.kind() {
                    std::io::ErrorKind::PermissionDenied => {
                        tracing::warn!("permission denied: {:?}", e.path());
                    }
                    std::io::ErrorKind::NotFound => {
                        // File deleted mid-scan — normal on a live filesystem
                        tracing::debug!("not found (deleted mid-scan): {:?}", e.path());
                    }
                    _ => {
                        tracing::warn!("io error at {:?}: {}", e.path(), io_err);
                    }
                }
            }
            result.skipped_count += 1;
            // continue is implicit in for loop after the match arm
        }
    }
}
```

### JSON Output Pattern (established from Phase 2)

```rust
// src/commands/scan.rs — established pattern from summary.rs
pub fn run(path: &Path, config: &Config, json: bool) -> anyhow::Result<()> {
    let result = crate::fs_scan::scan_path(path, config);
    if json {
        crate::output::write_json(&result)?;
    } else {
        render_scan_table(&result, path);
    }
    Ok(())
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `walkdir` with `follow_links(true)` | `follow_links(false)` (explicit) | walkdir 2.x best practice | Prevents infinite loops on symlink cycles |
| `metadata().len()` for disk usage | `MetadataExt::blocks() * 512` | POSIX st_blocks always existed; Rust exposed it via MetadataExt | Accurate sparse file accounting |
| Count all DirEntry occurrences | `(dev, ino)` HashSet deduplication | Best practice in disk tools (dua-cli, erdtree, pdu) | Correct hardlink handling |
| Sort all entries for top-N | BinaryHeap<Reverse<size>> capped at N | Algorithmic standard | O(n log N) vs O(n log n), bounded memory |

**Deprecated/outdated:**
- `filter_map(|e| e.ok())` for error handling: acceptable for simple scripts, wrong for any tool that needs to report skipped paths.
- `is_loop()` on walkdir::Error: this method does NOT exist. Use `loop_ancestor().is_some()` instead.

---

## Open Questions

1. **Top-N scope in Phase 3**
   - What we know: CONTEXT.md says `analyze` top-N aggregation is added in Phase 5; Phase 3 stubs it
   - What's unclear: Should `ScanResult.largest_files` be an empty `Vec` stub in Phase 3, or omitted entirely until Phase 5?
   - Recommendation: Include the field as empty Vec<FileEntry> in Phase 3 so the JSON schema is stable. Phase 5 populates it.

2. **Symlink size accounting**
   - What we know: `follow_links(false)` means symlinks are reported as symlink DirEntries; `entry.metadata()` follows to target
   - What's unclear: Should dangling symlinks be counted in `skipped_count` or a separate `symlink_count`?
   - Recommendation: Count dangling symlinks in `skipped_count` for simplicity. Phase 3 requirements do not specify symlink-specific reporting.

3. **Directory physical size**
   - What we know: Directories have their own inode with `blocks()` allocation (typically 0–8 blocks for the directory entry itself)
   - What's unclear: Should directory blocks be included in `total_bytes`?
   - Recommendation: Exclude directory blocks from `total_bytes` (match `du` default behavior which only sums file allocations for `-s` mode, then separately accounts directories). This matches user expectation and matches how `du -sh` works in practice.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (cargo test) + assert_cmd 2.0 for integration |
| Config file | none — cargo discovers tests automatically |
| Quick run command | `cargo test -p freespace --lib 2>/dev/null` |
| Full suite command | `cargo test -p freespace 2>/dev/null` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SCAN-01 | scan returns total_bytes, file_count, dir_count | unit | `cargo test -p freespace fs_scan -- --nocapture` | ❌ Wave 0 |
| SCAN-02 | streaming traversal — no Vec collection | unit | `cargo test -p freespace scan_does_not_collect` | ❌ Wave 0 |
| SCAN-03 | hardlink deduplication via (dev, ino) | unit | `cargo test -p freespace hardlink_not_double_counted` | ❌ Wave 0 |
| SCAN-04 | physical size = blocks * 512, not len() | unit | `cargo test -p freespace sparse_file_physical_size` | ❌ Wave 0 |
| SCAN-05 | permission denied + broken symlink: no crash, skip counter increments | integration | `cargo test -p freespace --test scan_cmd` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p freespace --lib 2>/dev/null`
- **Per wave merge:** `cargo test -p freespace 2>/dev/null`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src/fs_scan/mod.rs` — module does not exist yet; must be created with unit tests inline
- [ ] `src/analyze/mod.rs` — stub module does not exist yet; ScanResult type needed before tests can compile
- [ ] `tests/scan_cmd.rs` — integration test file needed; covers SCAN-05 (error resilience)
- [ ] `tests/scan_cmd.rs` hardlink fixture: create with `std::fs::hard_link()` in tempfile dir for SCAN-03
- [ ] `tests/scan_cmd.rs` sparse file fixture: create sparse file via `File::set_len()` without writing data for SCAN-04

**Note on SCAN-02 (streaming test):** Direct verification that no Vec is collected is a compile-time/review concern, not a runtime test. The integration test for SCAN-01 implicitly verifies SCAN-02 if the test directory has 100k+ files — for Phase 3, the pattern review in the plan is sufficient.

---

## Sources

### Primary (HIGH confidence)

- `std::os::unix::fs::MetadataExt` — https://doc.rust-lang.org/std/os/unix/fs/trait.MetadataExt.html — confirmed method names: `blocks()`, `ino()`, `dev()`; blocks() is u64 in 512-byte units
- `walkdir::Error` — https://docs.rs/walkdir/latest/walkdir/struct.Error.html — confirmed methods: `loop_ancestor()`, `io_error()`, `path()`, `depth()`; NOTE: `is_loop()` does NOT exist
- walkdir basic usage — https://docs.rs/walkdir/latest/ — confirmed: `follow_links(false)` is default; `filter_map(|e| e.ok())` silently swallows all errors
- Existing project code — `src/commands/summary.rs`, `src/output/mod.rs`, `src/platform/macos.rs` — established patterns for table output, write_json(), and anyhow::Result usage
- PITFALLS.md — project research file — hardlink deduplication pattern, physical size pattern, TCC handling, symlink loop handling
- ARCHITECTURE.md — project research file — module structure, streaming iterator pattern, pipeline design

### Secondary (MEDIUM confidence)

- comfy-table 7.2 docs — https://docs.rs/comfy-table — Table::new(), set_header(), add_row() pattern; verified against working summary.rs implementation
- bytesize 2.3 — used in summary.rs; ByteSize::b(u64).to_string() produces human-readable output

### Tertiary (LOW confidence)

- APFS sparse file behavior: `blocks() * 512` gives physically allocated bytes; confirmed conceptually from PITFALLS.md APFS research; not independently verified via syscall-level documentation

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in Cargo.toml; method names verified from official stdlib docs
- Architecture: HIGH — follows CONTEXT.md constraints exactly; consistent with established patterns in phases 1-2
- Pitfalls: HIGH — directly sourced from project PITFALLS.md which cites official macOS/Rust sources
- walkdir error API: HIGH — verified from docs.rs; `is_loop()` correction is load-bearing (PITFALLS.md had the wrong method name)

**Research date:** 2026-03-29
**Valid until:** 2026-06-29 (stable crates; stdlib API stable)
