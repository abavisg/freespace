# Plan 07-01: Cleanup Apply — Summary

**Status:** Complete
**Completed:** 2026-04-11

## What Was Built

Implemented `freespace clean apply` — the only command with irreversible side effects. Expanded the stub in `src/commands/clean.rs` into a complete safety-first deletion pipeline.

## Tasks Completed

| Task | Wave | Description | Status |
|------|------|-------------|--------|
| Task 1 | 0 (RED) | Created `freespace/tests/clean_apply_cmd.rs` with 8 failing integration tests covering all 5 APPLY requirements | Complete |
| Task 2 | 1a | Session file infrastructure — `write_preview_session()`, `load_preview_session()` with 1h TTL, `state_dir()` with env-var override for test isolation | Complete |
| Task 3 | 1b | Full `run_apply()` pipeline — protected-path guard, network volume warn+skip, trash/force dispatch, JSON Lines audit log, y/N confirmation prompt | Complete |

## Key Files

- `freespace/tests/clean_apply_cmd.rs` — 8 integration tests, all passing
- `freespace/src/commands/clean.rs` — full implementation (session write in `run_preview`, full pipeline in `run_apply`)
- `freespace/Cargo.toml` — added `chrono` dependency

## Test Results

```
running 8 tests
test test_apply_expired_session_fails ... ok
test test_apply_no_session_fails ... ok
test test_apply_protected_path_never_deletes ... ok
test test_apply_network_volume_warned_and_skipped ... ok
test test_apply_json_mode_bypasses_prompt ... ok
test test_apply_trashes_safe_candidates ... ok
test test_apply_audit_log_written ... ok
test test_apply_force_required_for_permanent_delete ... ok

test result: ok. 8 passed; 0 failed
```

Full suite: all tests green, no regressions.

## Requirements Covered

- APPLY-01: `freespace clean apply` moves files to macOS Trash via `trash::TrashContext` with `NsFileManager` method
- APPLY-02: Permanent deletion only with `--force`; without it, trash is used
- APPLY-03: Protected paths (`/System`, `/usr`, `/bin`, `/sbin`, `/private`) blocked unconditionally via `is_protected()` before any deletion
- APPLY-04: All actions logged to `~/.local/state/Freespace/cleanup.log` as JSON Lines with `{timestamp, path, size_bytes, action}`
- APPLY-05: `run_apply()` loads session file from `~/.local/state/Freespace/preview-session.json`; fails with informative error if missing or older than 1 hour

## Decisions Honored

- Network volumes: warn + skip (`eprintln!("skipped: network volume — {path}")`)
- Confirmation: y/N prompt (default N), bypassed in `--json` mode
- Audit log: JSON Lines, unbounded, append-only
- Session TTL: 3600 seconds (1 hour)
