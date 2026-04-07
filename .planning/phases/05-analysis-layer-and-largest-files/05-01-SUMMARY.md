---
phase: 05-analysis-layer-and-largest-files
plan: 01
subsystem: scan
tags: [rust, binaryheap, hashmap, walkdir, comfy-table, bytesize, serde]

# Dependency graph
requires:
  - phase: 03-core-scan-engine
    provides: scan_path, FileEntry, ScanResult, hardlink dedup via seen_inodes
  - phase: 01-foundation
    provides: CLI scaffolding, output::write_json, Config schema

provides:
  - BinaryHeap top-N file aggregation bounded to DEFAULT_TOP_N=20
  - HashMap directory size rollup with ancestor chain propagation
  - ScanResult.largest_dirs field (Vec<FileEntry> with is_dir=true)
  - freespace largest <path> command — table and JSON output
  - 5 integration tests and 4 new unit tests for largest functionality

affects: [06-cache-discovery, 07-cleanup-preview, 08-cleanup-apply]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "BinaryHeap<Reverse<(u64, PathBuf)>> for memory-bounded top-N without full sort"
    - "HashMap<PathBuf, u64> for ancestor directory size rollup in single scan pass"
    - "into_sorted_vec() on BinaryHeap<Reverse<T>> yields descending order by original value"

key-files:
  created:
    - freespace/tests/largest_cmd.rs
  modified:
    - freespace/src/analyze/mod.rs
    - freespace/src/fs_scan/mod.rs
    - freespace/src/commands/largest.rs

key-decisions:
  - "BinaryHeap<Reverse<T>> min-heap: into_sorted_vec() gives descending order without extra .reverse() call"
  - "Directory rollup walks ancestor chain from file to root, capped by root starts_with guard"
  - "LargestResult is a separate struct from ScanResult to expose only relevant fields in JSON output"

patterns-established:
  - "Bounded top-N: push unconditionally until capacity, then compare with heap min before eviction"
  - "Ancestor rollup: while let Some(dir) = ancestor pattern with root boundary check"

requirements-completed: [SCAN-06]

# Metrics
duration: 3min
completed: 2026-04-02
---

# Phase 5 Plan 01: Analysis Layer and Largest Files Summary

**BinaryHeap top-N file tracking and HashMap directory rollup wired into scan_path, backed by freespace largest command with table and JSON output**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-04-02T00:00:00Z
- **Completed:** 2026-04-02
- **Tasks:** 2
- **Files modified:** 4 (3 modified, 1 created)

## Accomplishments

- Extended ScanResult with `largest_dirs: Vec<FileEntry>` field
- Implemented memory-bounded top-N file tracking via `BinaryHeap<Reverse<(u64, PathBuf)>>` inside scan_path
- Implemented directory size rollup via `HashMap<PathBuf, u64>` ancestor chain traversal, correctly deduplicating hardlinks
- Replaced stub `largest::run()` with full implementation: path validation, scan call, table rendering, JSON output
- 4 new unit tests (bounded heap, hardlink dedup, descending sort, ancestor subdirs) + 5 integration tests all pass
- Full test suite green: 63 unit tests + 24 integration tests (zero regressions)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add BinaryHeap top-N and directory rollup to scan engine** - `bd36b01` (feat)
2. **Task 2: Implement largest command rendering and integration tests** - `2d4d9d2` (feat)

## Files Created/Modified

- `freespace/src/analyze/mod.rs` — Added `largest_dirs: Vec<crate::fs_scan::FileEntry>` field to ScanResult
- `freespace/src/fs_scan/mod.rs` — Added DEFAULT_TOP_N constant, BinaryHeap file tracking, HashMap dir rollup, 4 unit tests
- `freespace/src/commands/largest.rs` — Full implementation replacing stub: LargestResult, render_largest_table, JSON output
- `freespace/tests/largest_cmd.rs` — 5 integration tests: basic, json, stderr-clean, missing-path, ordering

## Decisions Made

- `into_sorted_vec()` on `BinaryHeap<Reverse<T>>` returns elements in ascending Reverse order, which is descending by original value — no extra `.reverse()` call needed. Plan code had a `.reverse()` that would have reversed the order; fixed inline.
- `LargestResult` is a separate serialization struct from `ScanResult` to expose only `root`, `total_bytes`, `largest_files`, `largest_dirs` in JSON output (not `file_count`, `dir_count`, `skipped_count`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed incorrect `.reverse()` call after `into_sorted_vec()`**
- **Found during:** Task 1 (TDD GREEN — test `largest_files_sorted_descending` failed)
- **Issue:** Plan code called `.reverse()` after `into_sorted_vec()` on `BinaryHeap<Reverse<...>>`. `into_sorted_vec()` already returns in descending order by original size for a min-heap; the extra `.reverse()` produced ascending order, failing the sort test.
- **Fix:** Removed both `.reverse()` calls (for file heap and dir heap) — used `into_sorted_vec()` directly
- **Files modified:** freespace/src/fs_scan/mod.rs
- **Verification:** `largest_files_sorted_descending` test passes; all 10 fs_scan unit tests pass
- **Committed in:** bd36b01 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug in plan's sorting logic)
**Impact on plan:** Necessary for correct descending sort. No scope creep.

## Issues Encountered

None beyond the sorting deviation above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `freespace largest <path>` is fully functional with table and JSON output
- `largest_files` and `largest_dirs` bounded to 20 entries, sorted descending by physical size
- Hardlink deduplication verified at both unit and integration level
- Phase 6 (cache discovery) can build on scan_path and the established FileEntry/ScanResult types without modification

---
*Phase: 05-analysis-layer-and-largest-files*
*Completed: 2026-04-02*

## Self-Check: PASSED

All created files exist. Both task commits (bd36b01, 2d4d9d2) verified in git history. SUMMARY.md created. STATE.md and ROADMAP.md updated.
