# Phase 5: Analysis Layer and Largest Files — Research

**Researched:** 2026-04-02
**Domain:** Rust BinaryHeap bounded aggregation, directory-size rollup, comfy-table output, serde JSON
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SCAN-06 | `freespace largest <path>` reports top-N largest files and directories at a path | BinaryHeap<Reverse<(u64, PathBuf)>> pattern verified in Rust std; directory rollup via HashMap aggregation; comfy-table + bytesize already in Cargo.toml |
</phase_requirements>

---

## Summary

Phase 5 implements the `freespace largest <path>` subcommand. The CLI stub (`commands/largest.rs`) already exists and is wired in `main.rs` and `cli.rs`. The scan infrastructure (`fs_scan::scan_path`, `fs_scan::FileEntry`, `analyze::ScanResult`) is complete from Phase 3, with an explicit comment in `fs_scan/mod.rs` noting the exact insertion point for the BinaryHeap.

The core challenge is two-fold: (1) track the top-N largest individual **files** using a min-heap (BinaryHeap<Reverse<...>>) so that heap size never exceeds N regardless of directory depth; and (2) compute **directory subtree sizes** by aggregating per-file sizes into a HashMap keyed by ancestor paths, then similarly selecting the top-N from that map. Both operations must use physical size (blocks * 512), respect hardlink deduplication, and reuse the existing scan loop rather than introducing a second traversal.

The output contract matches prior commands: human-readable `comfy-table` table by default, clean JSON via `--json` with `output::write_json`, all errors to stderr only, exit 0 on success.

**Primary recommendation:** Implement aggregation entirely inside `fs_scan::scan_path` (already has the `// Phase 5: update largest_files BinaryHeap here` hook) and add a parallel `largest_dirs: Vec<FileEntry>` field to `analyze::ScanResult`. Expose a standalone `largest::run()` that calls `scan_path` and renders from these fields. This keeps the scan a single-pass O(N log k) operation.

---

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `std::collections::BinaryHeap` | stdlib | Bounded top-N heap | No external dep; O(log k) per insert where k = top-N limit |
| `std::cmp::Reverse` | stdlib | Convert max-heap to min-heap | Standard Rust pattern for top-N selection |
| `walkdir` | 2.5 (already in Cargo.toml) | Directory traversal | Already used by all other commands |
| `bytesize` | 2.3 (already in Cargo.toml) | Human-readable byte formatting | Already used by scan and categories commands |
| `comfy-table` | 7.2 (already in Cargo.toml) | Table rendering | Already used by scan and categories commands |
| `serde` + `serde_json` | 1.0 (already in Cargo.toml) | JSON serialization | Already used everywhere |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `std::collections::HashMap` | stdlib | Directory size aggregation | Map each file's ancestor dirs to cumulative size |

**No new dependencies required.** Everything needed is already in `Cargo.toml`.

---

## Architecture Patterns

### How the Existing Codebase is Structured

The scan pipeline is:

```
fs_scan::scan_path(root, config)  →  analyze::ScanResult
                                          ↓
commands::largest::run(path, config, json)  →  renders ScanResult
```

`ScanResult` already has `largest_files: Vec<FileEntry>` typed correctly (Phase 3 decision: "ScanResult.largest_files typed as Vec<FileEntry> from the start — Phase 5 populates via BinaryHeap without breaking type change").

### Recommended Extension to ScanResult

Add `largest_dirs: Vec<FileEntry>` to `analyze::ScanResult`:

```rust
// src/analyze/mod.rs
#[derive(Debug, Default, Serialize)]
pub struct ScanResult {
    pub root: PathBuf,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub skipped_count: u64,
    pub largest_files: Vec<FileEntry>,  // already exists
    pub largest_dirs: Vec<FileEntry>,   // add in Phase 5
}
```

### Pattern 1: Bounded Top-N with BinaryHeap<Reverse<...>>

The canonical Rust idiom for top-N selection without sorting the full set:

```rust
// Source: Rust std docs — BinaryHeap
use std::collections::BinaryHeap;
use std::cmp::Reverse;

const TOP_N: usize = 20;

// Inside the scan loop, for each unique file:
// heap is BinaryHeap<Reverse<(u64, PathBuf)>>
// It's a min-heap via Reverse: the smallest element is at the top.
// We only keep TOP_N entries; when full, push + pop evicts the smallest.
if heap.len() < TOP_N {
    heap.push(Reverse((physical, path.to_path_buf())));
} else if let Some(&Reverse((min_size, _))) = heap.peek() {
    if physical > min_size {
        heap.pop();
        heap.push(Reverse((physical, path.to_path_buf())));
    }
}

// After scan: drain heap into Vec sorted largest-first
let mut largest: Vec<_> = heap.into_sorted_vec(); // sorted ascending (Reverse)
largest.reverse(); // now descending
```

`into_sorted_vec()` on a `BinaryHeap<Reverse<T>>` returns elements smallest-first (because Reverse inverts the order). Calling `.reverse()` produces the final largest-first slice.

### Pattern 2: Directory Size Rollup via Ancestor Walk

For each file encountered during the walk, walk up its ancestor chain and add the file's physical size to every ancestor's running total:

```rust
// Source: derived from standard Rust path ancestor iteration
use std::collections::HashMap;

let mut dir_sizes: HashMap<PathBuf, u64> = HashMap::new();

// Inside the scan loop, after computing `physical` for a file at `entry_path`:
let mut current = entry_path.parent();
while let Some(ancestor) = current {
    if !ancestor.starts_with(root) {
        break; // don't accumulate above the scan root
    }
    *dir_sizes.entry(ancestor.to_path_buf()).or_insert(0) += physical;
    current = ancestor.parent();
}
```

After the scan, select top-N from `dir_sizes` using the same BinaryHeap pattern.

**Memory bound:** The `dir_sizes` HashMap will contain at most one entry per unique directory in the scan tree. This is bounded by the number of directories, which is typically orders of magnitude smaller than the number of files. It is NOT bounded by TOP_N (it grows with directory count). This is acceptable — the requirement says the file heap is bounded; directory map is bounded by directory count, which is fine.

### Pattern 3: Rendering (follows existing command pattern)

```rust
// Follow the exact same pattern as commands/categories.rs
use comfy_table::Table;
use bytesize::ByteSize;

fn render_largest_table(files: &[FileEntry], dirs: &[FileEntry], root: &Path) {
    // Files table
    let mut table = Table::new();
    table.set_header(vec!["Path", "Size", "Type"]);
    for f in files {
        table.add_row(vec![
            f.path.display().to_string(),
            ByteSize::b(f.size).to_string(),
            if f.is_dir { "dir" } else { "file" }.to_string(),
        ]);
    }
    println!("Largest items in: {}", root.display());
    println!("{table}");
}
```

For JSON output, define a result struct and call `crate::output::write_json(&result)`.

### Recommended Project Structure (Phase 5 touches)

```
src/
├── analyze/mod.rs        # Add largest_dirs: Vec<FileEntry> to ScanResult
├── fs_scan/mod.rs        # Fill in the BinaryHeap hook; add dir_sizes aggregation
├── commands/largest.rs   # Implement run() — scan, render table or JSON
tests/
└── largest_cmd.rs        # New integration test file (see Wave 0 Gaps)
```

### Anti-Patterns to Avoid

- **Full sort instead of BinaryHeap:** Collecting all FileEntry into a Vec and sorting is O(N log N) in memory. A bounded heap is O(N log k) where k = TOP_N. Do not sort the full file list.
- **Separate second walk for directories:** Do not call `WalkDir` twice (once for files, once for dirs). Accumulate dir sizes inside the single existing scan loop.
- **Using logical size (`metadata().len()`):** The codebase decision is physical size (`blocks() * 512`) everywhere. Phase 5 must follow this.
- **Counting hardlinked files multiple times in dir rollup:** The `seen_inodes` HashSet is already used; only add to `dir_sizes` after the `seen_inodes.insert(key)` guard confirms this is the first time the inode is seen.
- **Including dirs above the scan root in dir_sizes:** The ancestor walk must break when `ancestor.starts_with(root)` is false.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Human-readable byte sizes | Custom format fn | `bytesize::ByteSize` | Already in Cargo.toml; handles GiB/MiB/KiB correctly |
| Terminal table | Manual string formatting | `comfy-table` | Already in Cargo.toml; consistent with all other commands |
| JSON serialization | Manual string building | `serde_json` via `output::write_json` | Already the project standard; consistent field naming |
| Top-N selection | Sort full vec + truncate | `BinaryHeap<Reverse<...>>` | Memory-bounded; this is the explicit success criterion |
| Directory traversal | Custom readdir recursion | `walkdir` | Already used; handles symlink loops, permission errors |

---

## Common Pitfalls

### Pitfall 1: Sorting instead of Heaping
**What goes wrong:** Developer collects all entries into a Vec, sorts by size descending, takes `.iter().take(N)`. This works but violates the explicit success criterion ("uses a bounded BinaryHeap, not a full sort").
**Why it happens:** Sorting is simpler to write and reason about.
**How to avoid:** Use `BinaryHeap<Reverse<(u64, PathBuf)>>` with a capacity check on every insert.
**Warning signs:** `vec.sort_by(...)` anywhere in the aggregation path.

### Pitfall 2: Double-counting hardlinks in directory rollup
**What goes wrong:** A hardlinked file appears twice in the walkdir stream. Both appearances walk up ancestor dirs and add size — inflating directory totals.
**Why it happens:** The `seen_inodes` guard only runs once and must be checked before accumulating into `dir_sizes`.
**How to avoid:** Only call the ancestor accumulation loop after `seen_inodes.insert(key)` returns `true` (i.e., first-seen inode).
**Warning signs:** Directory total exceeds total_bytes in ScanResult.

### Pitfall 3: PathBuf cloning cost in tight loop
**What goes wrong:** Every ancestor creates a new PathBuf allocation. For deeply nested trees this can be significant.
**Why it happens:** `ancestor.to_path_buf()` is called in the inner loop.
**How to avoid:** This is acceptable for Phase 5 (correctness first). The key is correctness. Note it as a potential future optimization (Phase 8 or PERF-01).
**Warning signs:** Not a correctness issue; a performance concern for v2.

### Pitfall 4: Forgetting to add `largest_dirs` to JSON output
**What goes wrong:** `--json` output only shows `largest_files`, missing directories. Success criterion requires both.
**Why it happens:** FileEntry.is_dir flag already exists but may be overlooked in JSON struct.
**How to avoid:** Define a `LargestResult` struct with both `largest_files` and `largest_dirs` fields, both serialized.

### Pitfall 5: top-N not configurable — hardcoded magic number
**What goes wrong:** Hardcoding `20` makes the command inflexible. Future phases or tests may want a different N.
**Why it happens:** "top-N" is specified without a concrete N in the requirements.
**How to avoid:** Define `const DEFAULT_TOP_N: usize = 20` at the module level in `commands/largest.rs` or `fs_scan/mod.rs`. This is readable and easy to change without breaking other modules.

---

## Code Examples

### Inserting into a bounded min-heap

```rust
// Source: Rust std BinaryHeap docs + canonical top-N pattern
use std::collections::BinaryHeap;
use std::cmp::Reverse;

let mut heap: BinaryHeap<Reverse<(u64, PathBuf)>> = BinaryHeap::new();
const TOP_N: usize = 20;

// Called for each unique file (after inode dedup):
fn maybe_insert(heap: &mut BinaryHeap<Reverse<(u64, PathBuf)>>, size: u64, path: PathBuf) {
    if heap.len() < TOP_N {
        heap.push(Reverse((size, path)));
    } else if heap.peek().map_or(false, |Reverse((min, _))| size > *min) {
        heap.pop();
        heap.push(Reverse((size, path)));
    }
}

// After scan — produces Vec<(u64, PathBuf)> sorted largest-first:
fn drain_heap_largest_first(heap: BinaryHeap<Reverse<(u64, PathBuf)>>) -> Vec<(u64, PathBuf)> {
    let mut v: Vec<_> = heap.into_sorted_vec(); // ascending by Reverse order
    v.reverse();
    v.into_iter().map(|Reverse(item)| item).collect()
}
```

### Converting heap output to Vec<FileEntry>

```rust
// Source: derived from FileEntry definition in fs_scan/mod.rs
use crate::fs_scan::FileEntry;

fn heap_to_file_entries(
    heap: BinaryHeap<Reverse<(u64, PathBuf)>>,
    is_dir: bool,
) -> Vec<FileEntry> {
    let mut sorted = heap.into_sorted_vec();
    sorted.reverse();
    sorted
        .into_iter()
        .map(|Reverse((size, path))| FileEntry { path, size, is_dir })
        .collect()
}
```

### Integration test skeleton (matches project pattern)

```rust
// tests/largest_cmd.rs — follows pattern established in tests/scan_cmd.rs
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn freespace() -> Command {
    Command::cargo_bin("freespace").unwrap()
}

#[test]
fn test_largest_basic() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("big.dat"), vec![0u8; 8192]).unwrap();
    fs::write(dir.path().join("small.dat"), b"tiny").unwrap();
    let output = freespace()
        .args(["largest", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.is_empty(), "largest must produce output");
}

#[test]
fn test_largest_json() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.dat"), vec![0u8; 4096]).unwrap();
    let output = freespace()
        .args(["--json", "largest", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout must be valid JSON");
    assert!(parsed.get("largest_files").is_some(), "must have largest_files");
    assert!(parsed.get("largest_dirs").is_some(), "must have largest_dirs");
}

#[test]
fn test_largest_stderr_clean_with_json() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("x.dat"), b"data").unwrap();
    let output = freespace()
        .args(["--json", "largest", dir.path().to_str().unwrap()])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "stderr must be empty with RUST_LOG=off and --json"
    );
}

#[test]
fn test_largest_missing_path() {
    let output = freespace()
        .args(["largest", "/nonexistent/path"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(!output.status.success(), "must exit non-zero for missing path");
}
```

---

## Implementation Decision: Single-Pass vs Two-Pass

The existing `fs_scan::scan_path` is the correct place to add BinaryHeap and dir aggregation. Evidence:

1. The comment at line 56 of `fs_scan/mod.rs` says: `// Phase 5: update largest_files BinaryHeap here`
2. `ScanResult.largest_files: Vec<FileEntry>` is already typed and stubbed (currently empty Vec)
3. The scan loop already has the `seen_inodes` dedup guard at exactly the right point

**Decision:** Add BinaryHeap logic and dir aggregation to `fs_scan::scan_path` directly. Do NOT add a second traversal in `commands/largest.rs`. The `largest` command calls `scan_path` and renders from the populated `ScanResult` fields.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness + assert_cmd 2.0 |
| Config file | Cargo.toml (dev-dependencies) |
| Quick run command | `cargo test -p freespace --test largest_cmd` |
| Full suite command | `cargo test -p freespace` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SCAN-06 | `largest <path>` exits 0 and produces output | integration | `cargo test -p freespace --test largest_cmd test_largest_basic` | No — Wave 0 gap |
| SCAN-06 | `--json` produces valid JSON with `largest_files` and `largest_dirs` | integration | `cargo test -p freespace --test largest_cmd test_largest_json` | No — Wave 0 gap |
| SCAN-06 | stderr is empty with `--json` and `RUST_LOG=off` | integration | `cargo test -p freespace --test largest_cmd test_largest_stderr_clean_with_json` | No — Wave 0 gap |
| SCAN-06 | Missing path exits non-zero | integration | `cargo test -p freespace --test largest_cmd test_largest_missing_path` | No — Wave 0 gap |
| SCAN-06 | BinaryHeap size stays bounded at TOP_N | unit | `cargo test -p freespace --lib -- fs_scan::tests::bounded_heap_does_not_exceed_top_n` | No — Wave 0 gap |
| SCAN-06 | Dir sizes do not double-count hardlinks | unit | `cargo test -p freespace --lib -- fs_scan::tests::dir_size_hardlink_dedup` | No — Wave 0 gap |

### Sampling Rate
- **Per task commit:** `cargo test -p freespace --test largest_cmd`
- **Per wave merge:** `cargo test -p freespace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `tests/largest_cmd.rs` — integration tests covering SCAN-06 (4 tests above)
- [ ] Unit tests in `fs_scan/mod.rs` — `bounded_heap_does_not_exceed_top_n`, `dir_size_hardlink_dedup`

---

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — this phase is pure Rust code using stdlib and already-installed crates)

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `ncdu`-style load-all-into-memory | Streaming + bounded heap | Phase 3 decision | Memory use is O(k) for files, O(dirs) for directory rollup |
| Separate scan for largest vs summary | Single `scan_path` populates all fields | Phase 3/5 architecture decision | One pass, consistent results |

---

## Open Questions

1. **What is the concrete value of TOP_N?**
   - What we know: Requirements say "top-N" but don't give a default N
   - What's unclear: Should it be 10? 20? Configurable via `--top` flag?
   - Recommendation: Default to 20 (matches `ncdu` and `du`-style tools). Do NOT add a CLI flag in Phase 5 — keep the interface minimal. The constant can be promoted to a CLI arg in a later phase.

2. **Should `freespace largest` also call the scan's dir/file counts, or only list top-N?**
   - What we know: `ScanResult` already carries `total_bytes`, `file_count`, `dir_count`
   - What's unclear: Whether the table header shows overall stats or only the top-N list
   - Recommendation: Show the top-N list only, with a header line showing the root path and total bytes (mirrors `scan` command output style). Total bytes line adds context without clutter.

3. **Should `scan` command output also start showing `largest_files` now?**
   - What we know: `ScanResult.largest_files` is serialized via serde — once populated it will appear in `freespace scan --json` output
   - What's unclear: Is this desirable, or should `scan` continue to omit the list?
   - Recommendation: Once Phase 5 populates `largest_files` inside `scan_path`, it will automatically appear in `scan --json`. This is a feature, not a bug — it improves `scan` output. Keep it.

---

## Sources

### Primary (HIGH confidence)
- Rust std docs — `std::collections::BinaryHeap` and `std::cmp::Reverse`: https://doc.rust-lang.org/std/collections/struct.BinaryHeap.html
- Codebase inspection: `freespace/src/fs_scan/mod.rs` — explicit Phase 5 hook comment at line 56, `seen_inodes` dedup guard, physical size calculation
- Codebase inspection: `freespace/src/analyze/mod.rs` — `ScanResult` with `largest_files: Vec<FileEntry>` already typed
- Codebase inspection: `freespace/Cargo.toml` — no new dependencies required

### Secondary (MEDIUM confidence)
- Prior phase patterns (`categories.rs`, `scan.rs`, `summary.rs`) — integration test structure, output module usage, comfy-table/bytesize usage patterns

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in Cargo.toml, used by existing commands
- Architecture: HIGH — existing codebase has explicit Phase 5 hooks; pattern is clear from prior phases
- Pitfalls: HIGH — derived from direct codebase inspection of the scan loop and existing decisions
- Test patterns: HIGH — existing integration tests in `tests/` provide exact model to follow

**Research date:** 2026-04-02
**Valid until:** 2026-05-02 (stable Rust stdlib; no external dependencies changing)
