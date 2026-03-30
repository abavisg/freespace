---
phase: 04-classification-and-category-commands
plan: "01"
subsystem: classify
tags: [classification, category, commands, tdd]
dependency_graph:
  requires: []
  provides: [classify::Category, classify::SafetyClass, classify::classify_path, commands::categories]
  affects: [commands/categories.rs, main.rs]
tech_stack:
  added: []
  patterns: [tiered-classification, physical-size-blocks-512, hardlink-dedup-dev-ino, pre-init-all-categories]
key_files:
  created:
    - freespace/src/classify/mod.rs
    - freespace/tests/categories_cmd.rs
  modified:
    - freespace/src/commands/categories.rs
    - freespace/src/main.rs
decisions:
  - "is_hidden checks file_name dot prefix AND path_has_hidden_component to catch files inside hidden dirs like ~/.ssh/config"
  - "Category::all() pre-initializes HashMap so all 14 categories always appear in output even with zero counts"
  - "home_dir() resolved once outside walk loop for performance"
metrics:
  duration: 3min
  completed: "2026-03-30"
  tasks_completed: 2
  files_changed: 4
---

# Phase 04 Plan 01: Classification Engine and Categories Command Summary

Classification engine (classify/mod.rs) with all 14 categories, tiered path-first classification logic, safety classification, and fully-tested `freespace categories <path>` command with table and JSON output.

## What Was Built

### Task 1: classify module (feat(04-01) d6ec017)

Created `freespace/src/classify/mod.rs` with:

- `Category` enum: 14 variants (Video, Audio, Images, Documents, Archives, Applications, Developer, Caches, Mail, Containers, CloudSync, Hidden, SystemRelated, Unknown) — serde kebab-case, Display trait, `Category::all()` static slice
- `SafetyClass` enum: 4 variants (Safe, Caution, Dangerous, Blocked) — serde kebab-case, Display trait
- `classify_path(path, home)`: 6-tier priority: system paths > Trash > known macOS dirs > hidden/dotfiles > extension > Unknown
- `is_hidden(path)`: checks file_name for dot prefix
- `path_has_hidden_component(path, home)`: catches files inside hidden dirs (e.g., ~/.ssh/config)
- `classify_by_extension(path)`: maps 40+ extensions across 6 categories
- `safety_class(path, home)`: Blocked/Safe/Caution/Dangerous based on path
- 27 unit tests covering all specified behaviors

Added `mod classify;` to `freespace/src/main.rs`.

### Task 2: categories command (feat(04-01) b8dd5fc)

Replaced stub in `freespace/src/commands/categories.rs` with full implementation:

- Pre-initializes all 14 categories in HashMap (guarantees all appear in output)
- WalkDir traversal with hardlink dedup via `(dev, ino)` HashSet
- Physical size: `metadata.blocks() * 512`
- home_dir() resolved once before the walk loop
- JSON output: `CategoriesResult` with root, categories array, total_bytes, total_files
- Table output: comfy_table with Category | Size | Files columns, sorted by size descending, TOTAL summary row
- 4 integration tests in `freespace/tests/categories_cmd.rs`

## Verification Results

```
cargo test classify  →  27 passed, 0 failed
cargo test categories  →  4 passed, 0 failed
cargo build  →  clean (only pre-existing warnings)
cargo run -- --json categories /tmp  →  categories count: 14
```

Total: 74 tests passing across all test suites.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed hidden classification for files inside hidden directories**

- **Found during:** Task 1 (TDD RED→GREEN)
- **Issue:** `is_hidden(path)` only checked the file_name component. `~/.ssh/config` has file_name `config` (no dot prefix), so it was classified as Unknown instead of Hidden
- **Fix:** Added `path_has_hidden_component(path, home)` that strips the home prefix and checks if any remaining path component starts with `.`. Updated `classify_path()` Tier 4 to use `is_hidden(path) || path_has_hidden_component(path, home)`
- **Files modified:** freespace/src/classify/mod.rs
- **Commit:** d6ec017

## Self-Check: PASSED
