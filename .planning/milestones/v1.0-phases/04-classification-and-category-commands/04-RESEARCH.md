# Phase 4: Classification and Category Commands - Research

**Researched:** 2026-03-30
**Domain:** Rust classification engine + macOS path knowledge + walkdir traversal
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

None listed as hard-locked. All implementation choices are at Claude's discretion. The
following constraints come from the PRD and must be followed:

**Classification priority order (MUST be followed):**
1. Path rules (highest priority)
2. Known macOS dirs (~/Library/Caches → caches, ~/Library/Mail → mail, ~/.ollama → developer, etc.)
3. Extension mapping (video/audio/images/documents/archives/applications)
4. Fallback → unknown

**14 categories (ALL must be present):**
video, audio, images, documents, archives, applications, developer, caches, mail, containers, cloud-sync, hidden, system-related, unknown

**Safety classification for caches (safe/caution/dangerous/blocked):**
- safe: standard user cache dirs (~/Library/Caches/*)
- caution: app-specific caches that may break functionality if deleted
- dangerous: system-adjacent caches
- blocked: protected paths

**Architecture (MUST follow):**
- New module: `src/classify/mod.rs` — pure functions, no I/O, easily testable
- `classify::classify_path(path: &Path) -> Category` — main classification entry point
- `classify::safety_class(path: &Path) -> SafetyClass` — for caches command
- `commands/categories.rs`, `commands/hidden.rs`, `commands/caches.rs` — thin dispatch layers, stubs exist

**Key macOS known paths to classify:**
- ~/Library/Caches → caches
- ~/Library/Mail → mail
- ~/Library/Containers → containers
- ~/.Trash → unknown (never clean)
- ~/.ollama, ~/Library/Developer, DerivedData → developer
- ~/Library/CloudStorage, ~/.dropbox, ~/Library/Mobile Documents → cloud-sync
- /System, /usr, /bin → system-related
- Dotfiles/hidden → hidden

### Claude's Discretion

All implementation choices, including:
- Extension mapping approach (HashMap, match, or phf)
- Hidden file detection implementation
- Cache directory enumeration list
- Table formatting details

### Deferred Ideas (OUT OF SCOPE)

None — all classification features are in scope for this phase.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CAT-01 | `freespace categories <path>` groups disk usage into all 14 categories | classify module + scan_path traversal + per-entry classify_path() call |
| CAT-02 | Classification priority: path rules → known macOS dirs → extension → unknown | Tiered match in classify_path() with home_dir() prefix matching |
| CAT-03 | Each category entry shows total bytes and file count | HashMap<Category, (u64, u64)> accumulator during scan walk |
| HIDD-01 | `freespace hidden <path>` lists dotfiles and hidden dirs with individual sizes | is_hidden() check on file_name() starting with '.' |
| HIDD-02 | Hidden scan reports total hidden size | Running total accumulated while collecting hidden entries |
| CACH-01 | `freespace caches` discovers cache dirs across standard macOS locations | Enumerate known cache dirs, scan_path() each |
| CACH-02 | Each cache entry shows path, size, and safety classification | safety_class() applied to each discovered dir path |
| CACH-03 | Reclaimable total shown across all safe-classified caches | Filter entries by SafetyClass::Safe, sum sizes |
</phase_requirements>

---

## Summary

Phase 4 introduces the classification engine (`src/classify/mod.rs`) that all three new commands
depend on. The engine is a pure-function module — no I/O, no filesystem calls — making it trivially
unit-testable. The `categories` command uses the existing `fs_scan::scan_path()` traversal and calls
`classify_path()` per entry to accumulate per-category totals. The `hidden` command applies a simple
dotfile check during a similar walk. The `caches` command enumerates a hardcoded list of known macOS
cache locations, scans each, and applies `safety_class()`.

The `dirs` crate (v6.0.0, already in Cargo.toml) provides `dirs::home_dir()`, which is the correct
source of truth for `~` expansion. All path prefix matching must happen at runtime against the
resolved home dir — never against hardcoded `/Users/<name>` strings. The existing codebase already
uses `dirs::home_dir()` in `config/mod.rs` confirming the pattern.

All three commands follow the established command pattern: `anyhow::Result<()>`, `comfy-table` for
table output, `output::write_json()` for `--json`, `#[derive(Serialize)]` on result structs. No new
dependencies are needed for this phase.

**Primary recommendation:** Build `classify/mod.rs` first as a pure data module, validate it with
unit tests, then wire it into the three thin command handlers.

---

## Standard Stack

### Core (all already in Cargo.toml)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| walkdir | 2.5.0 | Directory traversal for categories + hidden | Already used in fs_scan; proven patterns in place |
| dirs | 6.0.0 | `home_dir()` for path prefix resolution | Already used in config/mod.rs; correct API for macOS |
| serde | 1.0.228 | `#[derive(Serialize)]` on result structs | All commands use this pattern |
| comfy-table | 7.2.2 | Table output | Established pattern in scan.rs |
| bytesize | 2.3.1 | Human-readable byte formatting | Used in scan.rs |
| anyhow | 1.0.102 | Error propagation in command handlers | All command handlers use this |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| match statement for extension mapping | `phf` crate (perfect hash, compile-time) | phf saves a dependency for a problem of ~50 extensions; match is readable and zero-cost at this scale |
| match statement for extension mapping | `HashMap` built at runtime | Adds runtime initialization cost; match is inlined by the compiler |
| `dirs::home_dir()` | `std::env::var("HOME")` | HOME can be unset or overridden; dirs::home_dir() is the safe path |

**Extension mapping verdict:** Use a `match` statement on `OsStr` (or `str::to_lowercase()`).
At ~50 extensions, a `match` compiles to a jump table and adds zero heap allocations.
No new dependency needed.

**Installation:** No new dependencies. All packages are already resolved.

**Version verification (confirmed via `cargo tree`):**
- dirs 6.0.0 — published, active
- walkdir 2.5.0 — stable, no breaking changes expected
- bytesize 2.3.1 — confirmed in tree

---

## Architecture Patterns

### Recommended Project Structure (additions only)

```
freespace/src/
├── classify/
│   └── mod.rs           # Category enum, SafetyClass enum, classify_path(), safety_class()
├── commands/
│   ├── categories.rs    # Thin handler: walk + classify + aggregate + render (stub exists)
│   ├── hidden.rs        # Thin handler: walk + is_hidden check + render (stub exists)
│   └── caches.rs        # Thin handler: enumerate known dirs + scan + classify + render (stub exists)
```

### Pattern 1: Category Enum

**What:** A plain Rust enum with all 14 variants, deriving `Debug`, `Clone`, `Copy`, `PartialEq`,
`Eq`, `Hash`, `Serialize`. `Hash` is required because it will be used as a `HashMap` key.

**When to use:** Every call to `classify_path()` returns this enum. The `categories` command
accumulates into `HashMap<Category, CategoryEntry>`.

```rust
// Source: project conventions (derive pattern from analyze/mod.rs and platform/macos.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Category {
    Video,
    Audio,
    Images,
    Documents,
    Archives,
    Applications,
    Developer,
    Caches,
    Mail,
    Containers,
    CloudSync,
    Hidden,
    SystemRelated,
    Unknown,
}
```

The JSON serialization key should use `serde(rename_all = "kebab-case")` so it matches the
14 category names specified in the PRD (`cloud-sync`, `system-related`).

### Pattern 2: SafetyClass Enum

**What:** Four variants covering the cache safety classification spec.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SafetyClass {
    Safe,
    Caution,
    Dangerous,
    Blocked,
}
```

### Pattern 3: classify_path() — Tiered Match

**What:** Pure function, no I/O, takes a `&Path` and a pre-resolved `home: &Path` (so
`dirs::home_dir()` is called once by the caller, not on every file).

```rust
// Source: pattern from project CONTEXT.md + Pitfall 8 (classification priority order)
pub fn classify_path(path: &Path, home: &Path) -> Category {
    // Tier 1: path-prefix rules (highest priority)
    if path.starts_with(home.join(".Trash")) {
        return Category::Unknown;
    }
    if path.starts_with("/System") || path.starts_with("/usr") || path.starts_with("/bin") || path.starts_with("/sbin") || path.starts_with("/private") {
        return Category::SystemRelated;
    }
    // Tier 2: known macOS dirs under home
    if path.starts_with(home.join("Library/Caches")) {
        return Category::Caches;
    }
    if path.starts_with(home.join("Library/Mail")) {
        return Category::Mail;
    }
    // ... other known dirs ...
    // Tier 3: hidden (dotfile)
    if is_hidden(path) {
        return Category::Hidden;
    }
    // Tier 4: extension
    classify_by_extension(path)
    // Tier 5: fallback
    // (classify_by_extension returns Category::Unknown for unmapped extensions)
}
```

**Key design note:** `home` is passed in, not computed inside `classify_path()`. This keeps the
function pure and fast — no I/O, no `Option` unwrap, and unit tests can pass synthetic `home`
values without touching the real filesystem.

### Pattern 4: is_hidden() Check

**What:** Checks whether the last path component (filename) starts with `.`.

```rust
pub fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}
```

**Important:** For the `hidden` command, the walk must check both files AND directories. A hidden
directory like `.ssh` should be listed with its total size (sum of contents), not just its own
directory entry size. The categories command uses is_hidden as a tier in classify_path(), while the
hidden command uses it as a direct filter during the walk.

### Pattern 5: categories.rs Command Structure

```rust
// Source: scan.rs pattern (same project)
pub fn run(path: &Path, config: &Config, json: bool) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("path does not exist: {}", path.display());
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    let mut totals: HashMap<Category, CategoryEntry> = HashMap::new();
    // initialize all 14 entries so zero-count categories appear in output
    for cat in Category::all() {
        totals.insert(cat, CategoryEntry { category: cat, total_bytes: 0, file_count: 0 });
    }
    // walk + classify
    for entry_result in WalkDir::new(path).follow_links(false) {
        match entry_result {
            Ok(entry) if entry.file_type().is_file() => {
                let meta = entry.metadata()?;
                let size = meta.blocks() * 512;
                let cat = classify::classify_path(entry.path(), &home);
                let e = totals.get_mut(&cat).unwrap();
                e.total_bytes += size;
                e.file_count += 1;
            }
            _ => {}
        }
    }
    // render
}
```

**Important for CAT-01:** All 14 categories MUST appear in output even with zero counts.
Pre-initialize the HashMap with all variants before the walk.

### Pattern 6: caches.rs — Known Locations Enumeration

**What:** The `caches` command does NOT take a `<path>` argument. It enumerates a hardcoded list of
cache locations, scans each with `scan_path()`, and applies `safety_class()`.

```rust
pub fn known_cache_dirs(home: &Path) -> Vec<(PathBuf, SafetyClass)> {
    vec![
        // safe: standard user cache, expected to be cleared
        (home.join("Library/Caches"), SafetyClass::Safe),
        // caution: app-specific support caches
        (home.join("Library/Application Support"), SafetyClass::Caution),
        // caution: Xcode derived data — large, safe to clear but breaks incremental builds
        (home.join("Library/Developer/Xcode/DerivedData"), SafetyClass::Caution),
        // caution: npm cache
        (home.join(".npm"), SafetyClass::Caution),
        // caution: cargo registry cache
        (home.join(".cargo/registry"), SafetyClass::Caution),
        // caution: pip cache
        (home.join("Library/Caches/pip"), SafetyClass::Safe), // subpath of safe
        // dangerous: docker volumes
        (home.join("Library/Containers/com.docker.docker"), SafetyClass::Dangerous),
    ]
}
```

**For CACH-03:** After scanning, filter entries where `safety_class == SafetyClass::Safe` and sum
`total_bytes` for the reclaimable total.

### Anti-Patterns to Avoid

- **Calling `dirs::home_dir()` inside `classify_path()`:** Makes the function impure (I/O),
  slower (called once per file), and hard to unit-test. Call it once in the command handler.
- **Checking extension before path rules:** Extension `.raw` would classify docker images as
  `Images`. Path rules must take strict precedence (Pitfall 8 from PITFALLS.md).
- **Omitting zero-count categories from output:** CAT-01 requires ALL 14 categories. Pre-initialize.
- **Walking hidden directories recursively for the `hidden` command:** A hidden directory entry
  should show the total size of the directory subtree, not be recursed into as individual files.
  Detect hidden at the top level then sum its size via a nested scan.
- **Using `metadata().len()` instead of `blocks() * 512`:** Physical size convention is established
  in fs_scan/mod.rs; classification output must match.
- **`safety_class()` as a pure path function returning hardcoded rules only:** The safety
  classification IS purely path-based for this phase, which is correct. No dynamic rules needed yet.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Home dir resolution | Custom `$HOME` env parsing | `dirs::home_dir()` | Handles edge cases (no HOME var, root user, non-standard setups) |
| Byte size formatting | Custom GiB/MiB formatter | `bytesize::ByteSize::b(n).to_string()` | Already used in scan.rs; consistent output |
| Table rendering | Manual column-aligned print! | `comfy_table::Table` | Already used in scan.rs; handles terminal width |
| JSON output | `println!("{}", serde_json::...)` | `output::write_json()` | Project-standard; writes to stdout lock |
| Directory traversal | Custom `std::fs::read_dir` recursion | `walkdir::WalkDir` | Already in fs_scan; handles errors, symlinks, loop detection |

**Key insight:** The classification engine itself must be custom (it encodes macOS domain knowledge),
but everything around it (traversal, output, path resolution) has established solutions already in
the codebase.

---

## Common Pitfalls

### Pitfall 1: Extension Match Before Path Rules (Pitfall 8 from PITFALLS.md)

**What goes wrong:** `~/Library/Application Support/Docker/*.raw` classifies as `Images`.
`~/.ollama/models/blobs/*` (no extension) classifies as `Unknown` instead of `Developer`.

**Why it happens:** Extension rules run first. Without explicit path rules for Docker and `.ollama`,
the extension match wins.

**How to avoid:** Strict ordering in `classify_path()`. Path rules ALWAYS check before extension.
Add unit tests with these exact paths to verify before shipping.

**Warning signs:** `freespace categories ~/Library` shows large `Images` or `Documents` entries that
should be `Developer` or `Caches`.

### Pitfall 2: home_dir() Called Per File

**What goes wrong:** Performance degrades on large trees; also makes `classify_path()` impure and
untestable.

**Why it happens:** Convenience — calling `dirs::home_dir()` inside classify_path() feels natural.

**How to avoid:** Call `dirs::home_dir()` once in the command handler, pass `home: &Path` into
`classify_path()`. Unit tests can then use `Path::new("/Users/testuser")` as a synthetic home.

### Pitfall 3: Hidden Directories Double-Counted

**What goes wrong:** The `hidden` command walks inside `.ssh/`, counts individual files as hidden,
then also counts `.ssh` itself. Total hidden size is inflated.

**Why it happens:** Naive `is_hidden(entry.path())` check in a standard WalkDir loop.

**How to avoid:** For the `hidden` command, detect hidden at the top level (immediate children of
the scanned root that start with `.`), then use `scan_path()` on each hidden dir to get its total.
For files that are themselves hidden (dotfiles), add them individually. Do not recurse inside a
hidden directory while also counting the directory itself.

### Pitfall 4: Zero-Count Categories Missing from Output

**What goes wrong:** `freespace categories /tmp` might show only 1-2 categories. CAT-01 requires
ALL 14.

**Why it happens:** HashMap only has keys for categories that were matched.

**How to avoid:** Pre-initialize the HashMap (or Vec) with all 14 Category variants before the walk.

### Pitfall 5: caches Command Scanning Nonexistent Dirs

**What goes wrong:** Many cache dirs may not exist (no Xcode on the machine, no npm, etc.). Scanning
a nonexistent path with `scan_path()` or WalkDir returns an error or empty result, but the error
path must be handled gracefully — caches should silently skip nonexistent dirs, not bail.

**Why it happens:** Enumerated cache dirs are hardcoded; not all will exist on every machine.

**How to avoid:** Check `path.exists()` before scanning each cache dir. Skip (don't error) if the
dir does not exist.

### Pitfall 6: .Trash Misclassified

**What goes wrong:** `.Trash` starts with `.` so `is_hidden()` returns true. It would fall into the
`Hidden` category. But CONTEXT.md says `.Trash` → `Unknown` (never clean).

**Why it happens:** Path rules that cover `.Trash` must appear before the hidden tier check.

**How to avoid:** Add `~/.Trash` as an explicit path rule (returning `Category::Unknown`) that runs
before `is_hidden()` is checked.

---

## Code Examples

Verified patterns from existing codebase:

### Category enum with serde rename (derived from project conventions)

```rust
// Source: project conventions — same derive chain as ScanResult in analyze/mod.rs
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Video,
    Audio,
    Images,
    Documents,
    Archives,
    Applications,
    Developer,
    Caches,
    Mail,
    Containers,
    CloudSync,    // serializes as "cloud-sync"
    Hidden,
    SystemRelated,  // serializes as "system-related"
    Unknown,
}

impl Category {
    pub fn all() -> &'static [Category] {
        use Category::*;
        &[Video, Audio, Images, Documents, Archives, Applications,
          Developer, Caches, Mail, Containers, CloudSync, Hidden,
          SystemRelated, Unknown]
    }
}
```

### Extension mapping (match approach)

```rust
// Source: project discretion — no external dep needed at this scale
fn classify_by_extension(path: &Path) -> Category {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase);
    match ext.as_deref() {
        Some("mp4" | "mov" | "avi" | "mkv" | "m4v" | "wmv" | "flv" | "webm") => Category::Video,
        Some("mp3" | "aac" | "flac" | "wav" | "m4a" | "ogg" | "opus") => Category::Audio,
        Some("jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "webp" | "heic") => Category::Images,
        Some("pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx"
           | "txt" | "md" | "pages" | "numbers" | "keynote") => Category::Documents,
        Some("zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "dmg" | "pkg") => Category::Archives,
        Some("app" | "ipa" | "apk") => Category::Applications,
        _ => Category::Unknown,
    }
}
```

**Note on `.raw`:** Do NOT add `raw` to the Images match arm. Raw disk images (Docker, QEMU) use
`.raw` too. Without a path-rule override, `.raw` files fall through to `Unknown`, which is the safe
default.

### home_dir usage (from config/mod.rs pattern)

```rust
// Source: freespace/src/config/mod.rs — confirmed working pattern for dirs v6.0.0
let home = dirs::home_dir()
    .ok_or_else(|| anyhow::anyhow!("Cannot resolve home directory"))?;
// Then pass &home into classify_path()
```

### CategoryEntry struct for JSON

```rust
#[derive(Debug, Serialize)]
pub struct CategoryEntry {
    pub category: Category,
    pub total_bytes: u64,
    pub file_count: u64,
}
```

### CacheEntry struct for JSON

```rust
#[derive(Debug, Serialize)]
pub struct CacheEntry {
    pub path: PathBuf,
    pub total_bytes: u64,
    pub safety: SafetyClass,
}

#[derive(Debug, Serialize)]
pub struct CachesResult {
    pub entries: Vec<CacheEntry>,
    pub reclaimable_bytes: u64,  // sum of Safe entries only
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `dirs` v4/v5 API had `home_dir()` in a different location | `dirs` v6.0.0 — `dirs::home_dir()` unchanged, still the correct call | v6 released ~2024 | No API change; already used in codebase |
| Hardcoded `/Users/<name>` for macOS home | `dirs::home_dir()` at runtime | Always correct; important for non-standard user accounts | Avoids hardcoded username bugs |

**Deprecated/outdated:**
- `dirs::home_dir()` was briefly stabilized/unstabilized in older Rust std — the `dirs` crate
  provides a consistent, cross-platform version. The codebase already uses this correctly.

---

## Open Questions

1. **Hidden directory size calculation strategy**
   - What we know: The hidden command must list hidden items with individual sizes
   - What's unclear: Should a hidden directory show the sum of its recursive contents, or just
     its own metadata size (trivially small)?
   - Recommendation: Sum recursive contents. Users care about reclaim potential. Use `scan_path()`
     on each hidden directory to get its total_bytes.

2. **Scope of `freespace caches` enumeration**
   - What we know: Standard user locations (~/Library/Caches, DerivedData, ~/.npm, ~/.cargo/registry)
   - What's unclear: How deep to go in the safety classification — is `~/Library/Application Support`
     as a whole "caution", or should specific subdirs be enumerated?
   - Recommendation: Enumerate at the specific-subdir level where the size is meaningful and the
     safety is distinguishable. `~/Library/Caches` as a whole is `Safe`. Individual app subdirs
     under it can be surfaced as separate entries if they are large enough (this is a Phase 4
     simplification: treat the parent dir as one entry).

3. **classify_path for directories vs files**
   - What we know: `scan_path()` emits both files and dirs; categories.rs accumulates per-file
   - What's unclear: Should directory entries themselves be classified and counted?
   - Recommendation: Only classify and count files (not directories). Directory entries have no
     meaningful "size" in the physical sense (they are just nodes). Size is accumulated from files.
     This is consistent with how `scan_path()` currently counts `file_count`.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | `freespace/Cargo.toml` (no separate test config) |
| Quick run command | `cargo test -p freespace classify` |
| Full suite command | `cargo test -p freespace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CAT-01 | All 14 categories present in output | unit | `cargo test -p freespace classify::tests` | Wave 0 |
| CAT-02 | Path rules take priority over extension | unit | `cargo test -p freespace classify::tests::path_rule_beats_extension` | Wave 0 |
| CAT-02 | macOS known dirs correctly classified | unit | `cargo test -p freespace classify::tests::known_macos_dirs` | Wave 0 |
| CAT-03 | Category output has total_bytes and file_count | integration | `cargo test -p freespace categories_cmd` | Wave 0 |
| HIDD-01 | Hidden files listed with sizes | integration | `cargo test -p freespace hidden_cmd::test_hidden_basic` | Wave 0 |
| HIDD-02 | Total hidden size reported | integration | `cargo test -p freespace hidden_cmd::test_hidden_total` | Wave 0 |
| CACH-01 | caches command runs and discovers dirs | integration | `cargo test -p freespace caches_cmd::test_caches_exits_ok` | Wave 0 |
| CACH-02 | Each entry has path, size, safety class | integration | `cargo test -p freespace caches_cmd::test_caches_json_fields` | Wave 0 |
| CACH-03 | reclaimable_bytes shown and is >= 0 | integration | `cargo test -p freespace caches_cmd::test_caches_reclaimable` | Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p freespace 2>&1 | tail -5`
- **Per wave merge:** `cargo test -p freespace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `freespace/src/classify/mod.rs` — core classify module (entire file is new)
- [ ] `freespace/tests/categories_cmd.rs` — integration tests for `freespace categories <path>`
- [ ] `freespace/tests/hidden_cmd.rs` — integration tests for `freespace hidden <path>`
- [ ] `freespace/tests/caches_cmd.rs` — integration tests for `freespace caches`

Unit tests for `classify/mod.rs` live inside the module itself in `#[cfg(test)]` blocks, following
the pattern established in `fs_scan/mod.rs` and `platform/macos.rs`.

---

## Sources

### Primary (HIGH confidence)
- Project source: `freespace/src/fs_scan/mod.rs` — scan_path() implementation, established patterns
- Project source: `freespace/src/platform/macos.rs` — protected_paths(), is_protected() patterns
- Project source: `freespace/src/config/mod.rs` — `dirs::home_dir()` usage, confirmed for v6.0.0
- Project source: `freespace/src/commands/scan.rs` — command handler pattern (anyhow, comfy-table, bytesize)
- Project source: `freespace/Cargo.toml` + `cargo tree` output — confirmed dependency versions
- `.planning/phases/04-classification-and-category-commands/04-CONTEXT.md` — locked decisions
- `.planning/REQUIREMENTS.md` — requirement IDs and acceptance criteria
- `.planning/research/PITFALLS.md` — Pitfall 8 (classification priority), all scan-related pitfalls

### Secondary (MEDIUM confidence)
- `dirs` crate v6 documentation — `home_dir()` API unchanged from v5 (confirmed via codebase usage)

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dependencies confirmed via `cargo tree` in the actual project
- Architecture: HIGH — derived directly from existing codebase patterns + CONTEXT.md decisions
- Pitfalls: HIGH — sourced from project PITFALLS.md (already researched) + code analysis

**Research date:** 2026-03-30
**Valid until:** 2026-04-30 (stable Rust ecosystem; 30-day validity is conservative)
