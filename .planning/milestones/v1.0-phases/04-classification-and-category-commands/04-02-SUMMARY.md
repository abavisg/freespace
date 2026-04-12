---
phase: 04-classification-and-category-commands
plan: 02
subsystem: cli-commands
tags: [rust, walkdir, comfy-table, bytesize, serde, dirs, classify, fs_scan]

# Dependency graph
requires:
  - phase: 04-01
    provides: classify module with is_hidden(), safety_class(), SafetyClass enum
  - phase: 03-core-scan-engine
    provides: fs_scan::scan_path() for recursive directory sizing

provides:
  - hidden command: dotfile/hidden-dir listing at given path with individual sizes and total
  - caches command: macOS cache directory discovery with safety classification and reclaimable total

affects:
  - 05-preview-and-cleanup (safe/dangerous classification informs cleanup safety gates)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - read_dir (not WalkDir) for top-level-only enumeration to avoid double-counting hidden dirs
    - scan_path for recursive sizing of hidden directories and cache directories
    - known_cache_dirs() registry pattern for discoverable macOS cache paths
    - safety == SafetyClass::Safe filter for reclaimable computation

key-files:
  created:
    - freespace/tests/hidden_cmd.rs
    - freespace/tests/caches_cmd.rs
  modified:
    - freespace/src/commands/hidden.rs
    - freespace/src/commands/caches.rs

key-decisions:
  - "read_dir used for hidden command top-level enumeration (not WalkDir) — prevents double-counting hidden directory contents"
  - "known_cache_dirs() hardcoded list of 7 macOS cache paths; nonexistent paths skipped silently"
  - "Reclaimable total = sum of Safe-classified cache entries only (not Caution/Dangerous/Blocked)"
  - "Library/Logs added as Safe cache dir — log files are safely reclaimable"

patterns-established:
  - "Pattern: top-level enumeration via read_dir + recursive sizing via scan_path for hidden command"
  - "Pattern: known-dir registry + safety_class() classification for cache command"

requirements-completed:
  - HIDD-01
  - HIDD-02
  - CACH-01
  - CACH-02
  - CACH-03

# Metrics
duration: 15min
completed: 2026-03-30
---

# Phase 4 Plan 2: Hidden and Caches Commands Summary

**Hidden command lists dotfiles at top level using read_dir to avoid double-counting; caches command discovers 7 macOS cache dirs, classifies safety, and computes reclaimable total from Safe entries**

## Performance

- **Duration:** 15 min
- **Started:** 2026-03-30T13:10:00Z
- **Completed:** 2026-03-30T13:25:00Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Implemented `freespace hidden <path>`: enumerates immediate children only via read_dir, sizes hidden directories via scan_path, sizes hidden files via metadata.blocks()*512, sorts by size descending, supports JSON and table output
- Implemented `freespace caches`: discovers 7 known macOS cache paths (Library/Caches, Library/Logs, Xcode DerivedData, .npm, .cargo/registry, Docker containers, .gradle), skips nonexistent dirs, classifies safety via classify::safety_class(), computes reclaimable as sum of Safe entries
- 8 integration tests across both commands — all passing, no regressions in full suite

## Task Commits

Each task was committed atomically:

1. **Task 1: hidden command with dotfile listing and integration tests** - `4b3bebb` (feat)
2. **Task 2: caches command with safety classification and integration tests** - `dfe3e92` (feat)

## Files Created/Modified
- `freespace/src/commands/hidden.rs` - Full hidden command: read_dir enumeration, scan_path for dirs, physical file sizing, table/JSON output
- `freespace/src/commands/caches.rs` - Full caches command: known_cache_dirs(), safety classification, reclaimable computation, table/JSON output
- `freespace/tests/hidden_cmd.rs` - 4 integration tests: basic listing, JSON fields, total sum, missing path error
- `freespace/tests/caches_cmd.rs` - 4 integration tests: exit 0, JSON fields, reclaimable bounds, safety value validation

## Decisions Made
- Used `std::fs::read_dir` (not WalkDir) for hidden command's top-level enumeration — critical for avoiding double-counting (e.g., listing `.ssh` and then also `.ssh/id_rsa` separately)
- Added `Library/Logs` to known_cache_dirs as Safe — log files are commonly reclaimable on macOS
- Reclaimable bytes is strictly `SafetyClass::Safe` entries only, matching CACH-03 requirement

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 4 complete: classification module and all category commands (categories, hidden, caches) fully implemented
- Phase 5 (Preview and Cleanup): safety classification and reclaimable computation from this phase are ready to feed cleanup safety gates
- All Phase 4 requirements (HIDD-01, HIDD-02, CACH-01, CACH-02, CACH-03) fulfilled

---
*Phase: 04-classification-and-category-commands*
*Completed: 2026-03-30*
