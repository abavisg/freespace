---
phase: 01-foundation
plan: 02
subsystem: cli
tags: [rust, clap, toml, serde, dirs, tracing, tempfile]

# Dependency graph
requires:
  - phase: 01-foundation-01-01
    provides: "Compilable Rust skeleton with all stub modules"
provides:
  - "load_config() with real TOML file reading via home_dir, graceful missing-file handling"
  - "platform::macos protected_paths() with canonicalize+fallback and is_protected() predicate"
  - "--json global flag tests confirming propagation through nested clean > preview subcommand"
  - "output::write_json wired to clean.rs and doctor.rs commands"
  - "23 passing unit tests covering all 6 FOUND requirements"
affects: [02-scan, 03-summary, 04-classification, 07-cleanup-apply]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Config loading uses dirs::home_dir().join('.config/Freespace/config.toml') NOT dirs::config_dir()"
    - "Tests use tempfile + internal load_from_path helper for hermetic config parsing (avoids home dir dependency)"
    - "CLI tests use pattern matching instead of unwrap_err/expect_err to avoid Debug bound on Cli struct"
    - "protected_paths() calls canonicalize once at startup; fallback via unwrap_or_else never panics"

key-files:
  created: []
  modified:
    - freespace/src/config/mod.rs
    - freespace/src/platform/macos.rs
    - freespace/src/cli.rs
    - freespace/src/output/mod.rs
    - freespace/src/commands/clean.rs
    - freespace/src/commands/doctor.rs

key-decisions:
  - "Config path is home_dir/.config/Freespace/config.toml — dirs::config_dir() returns ~/Library/Application Support on macOS which is not PRD-mandated path"
  - "Use pattern match in help_exits_with_display_help_error test to extract clap error without requiring Debug on Cli struct (Rust stdlib unwrap_err/expect_err both need T: Debug)"
  - "clean.rs and doctor.rs stub handlers output JSON via output::write_json when json=true, keeping stdout clean"

patterns-established:
  - "Pattern: all command handlers accept (config: &Config, json: bool) — json routing is caller responsibility"
  - "Pattern: missing config file is silently handled with Config::default() + debug trace log"
  - "Pattern: protected_paths() returns canonicalized paths (resolves /tmp → /private/tmp on macOS)"

requirements-completed: [FOUND-01, FOUND-02, FOUND-03, FOUND-04, FOUND-05, FOUND-06]

# Metrics
duration: 9min
completed: 2026-03-28
---

# Phase 1 Plan 02: Foundation Implementation Summary

**Full config loader, protected-path canonicalization, and global --json wiring with 23 passing unit tests covering all 6 FOUND requirements**

## Performance

- **Duration:** 9 min
- **Started:** 2026-03-28T16:33:17Z
- **Completed:** 2026-03-28T16:42:31Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments
- Config loader reads ~/.config/Freespace/config.toml via dirs::home_dir(); missing file returns Config::default() gracefully; malformed TOML returns Err
- platform::macos expanded with 8 tests: /System, /usr, /private/tmp protection, /tmp→/private/tmp canonicalization, and fallback-without-panic verification
- --json global flag confirmed propagating through nested clean > preview subcommand (7 cli tests); output::write_json wired to clean.rs and doctor.rs

## Task Commits

Each task was committed atomically:

1. **Task 1: Config loader with real TOML file reading** - `145a1a9` (feat)
2. **Task 2: platform::macos protected-path tests** - `2da6e75` (feat)
3. **Task 3: cli/output tests and --json wiring** - `eceb1f6` (feat)

## Files Created/Modified
- `freespace/src/config/mod.rs` - Full load_config() using home_dir; 5 hermetic tests
- `freespace/src/platform/macos.rs` - Expanded from 3 to 8 tests; all protected-path cases covered
- `freespace/src/cli.rs` - 7 tests replaced stub; global --json propagation confirmed
- `freespace/src/output/mod.rs` - Added write_json_serializes_struct test (3 tests total)
- `freespace/src/commands/clean.rs` - JSON output wired via output::write_json
- `freespace/src/commands/doctor.rs` - JSON output wired via output::write_json

## Decisions Made
- Config path uses `dirs::home_dir().join(".config/Freespace/config.toml")` rather than `dirs::config_dir()` — on macOS, `config_dir()` maps to `~/Library/Application Support` which is not the PRD-mandated location
- CLI test for `--help` uses `match result { Err(err) => ... }` pattern instead of `unwrap_err()` because both `unwrap_err` and `expect_err` require `T: Debug`, which `Cli` does not implement
- Stub command handlers output a `not_implemented` JSON object when `json=true` to satisfy the integration smoke test (stdout clean, no eprintln contamination)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed help_exits_with_display_help_error test compile error**
- **Found during:** Task 3 (cli tests implementation)
- **Issue:** Plan's test used `expect_err()` which requires `T: Debug` — `Cli` struct has no `#[derive(Debug)]`, causing compile error E0277
- **Fix:** Replaced `expect_err()` with explicit `match result { Err(err) => ..., Ok(_) => panic!(...) }` pattern — functionally identical, no Debug required
- **Files modified:** freespace/src/cli.rs
- **Verification:** `cargo test` compiles and all 23 tests pass
- **Committed in:** eceb1f6 (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - compile-time bug in test code)
**Impact on plan:** Minimal — single test rewrite with identical behavior. No scope creep.

## Issues Encountered
- `expect_err()` / `unwrap_err()` in Rust stdlib both require `T: Debug` on the `Ok` type. The plan's suggested test code used `expect_err()` without considering this constraint. Resolved by using a `match` expression instead.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 6 FOUND requirements implemented and tested (23 tests, all green)
- Config loading, protected paths, and CLI skeleton are production-quality
- Phase 2 (Scan) can begin — scan module can use `config::load_config()`, `platform::macos::protected_paths()`, and `platform::macos::is_protected()` directly

---
*Phase: 01-foundation*
*Completed: 2026-03-28*

## Self-Check: PASSED

- freespace/src/config/mod.rs: FOUND
- freespace/src/platform/macos.rs: FOUND
- freespace/src/cli.rs: FOUND
- freespace/src/output/mod.rs: FOUND
- freespace/src/commands/clean.rs: FOUND
- freespace/src/commands/doctor.rs: FOUND
- .planning/phases/01-foundation/01-02-SUMMARY.md: FOUND
- Commit 145a1a9 (Task 1): FOUND
- Commit 2da6e75 (Task 2): FOUND
- Commit eceb1f6 (Task 3): FOUND
- Commit 2aae221 (metadata): FOUND
