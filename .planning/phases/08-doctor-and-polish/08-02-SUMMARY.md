---
phase: 08-doctor-and-polish
plan: "02"
subsystem: diagnostics
tags: [doctor, tcc, fda, comfy_table, serde, diagnostics, exit-codes]

requires:
  - phase: 08-01
    provides: completions subcommand, Wave 0 RED doctor test scaffold (doctor_cmd.rs)

provides:
  - Full doctor command implementation (4 checks: TCC/FDA, protected paths, config file, cleanup log)
  - Human-mode comfy_table output with Check/Status/Message columns and ✓/✗/⚠ symbols
  - JSON mode with checks array and overall field via write_json
  - Exit 0 on pass/warn, exit 1 via anyhow::bail on any fail check
  - DIAG-01 and DIAG-02 satisfied — Freespace v1 feature-complete

affects: [verify-work, phase-09]

tech-stack:
  added: []
  patterns:
    - TCC probe via std::fs::metadata on ~/Library/Safari/History.db (PermissionDenied=fail, FileNotFound=warn, Ok=pass)
    - DoctorCheck struct with CheckStatus enum serialized snake_case via serde
    - comfy_table with Table::new() — no preset, consistent with summary.rs and clean.rs patterns
    - anyhow::bail for non-zero exit (never std::process::exit — preserves buffer flushing)

key-files:
  created: []
  modified:
    - freespace/src/commands/doctor.rs

key-decisions:
  - "TCC probe uses Safari/History.db not NSWorkspace — avoids entitlement complexity, deterministic pass/fail/warn semantics"
  - "CheckStatus serialized with serde rename_all=snake_case — matches JSON contract tested in doctor_cmd.rs"
  - "anyhow::bail for fail exit — preserves stdout/stderr flushing vs std::process::exit"
  - "check_protected_paths uses cfg(target_os=macos) guard with non-macOS warn fallback — consistent with platform module pattern"
  - "Config check uses toml::Value (generic) not Config struct — malformed TOML fails check, valid TOML passes regardless of unknown keys"

requirements-completed: [DIAG-01, DIAG-02]

duration: 10min
completed: 2026-04-11
---

# Phase 8 Plan 02: Doctor and Polish Summary

**freespace doctor fully implemented: 4-check TCC/FDA diagnostic command with comfy_table human output, JSON mode, actionable remediation messages, and exit-code semantics (0=pass/warn, 1=fail)**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-04-11T22:00:00Z
- **Completed:** 2026-04-11T22:10:00Z
- **Tasks:** 2 (combined into single atomic commit)
- **Files modified:** 1

## Accomplishments

- Implemented `freespace doctor` with 4 checks: Full Disk Access (TCC probe), Protected paths (reuses platform::macos::protected_paths()), Config file (~/.config/Freespace/config.toml), Cleanup log (~/.local/state/Freespace/cleanup.log)
- Human-mode output: comfy_table with Check/Status/Message columns, ✓/✗/⚠ symbols per check, summary line after table
- JSON-mode output: `{"checks": [...], "overall": "pass|warn|fail"}` via crate::output::write_json
- Exit 0 when overall is pass or warn; exit 1 via anyhow::bail when any check has Fail status
- All 9 tests in doctor_cmd.rs GREEN; full cargo test suite (70 unit + 34 integration) passes with 0 failures

## Sample Output

**Human mode (`freespace doctor`):**
```
+------------------+--------+------------------------------------------------------+
| Check            | Status | Message                                              |
+======================================================================+
| Full Disk Access | ✓      | Granted                                              |
| Protected paths  | ✓      | 6/6 verified                                         |
| Config file      | ⚠      | ~/.config/Freespace/config.toml not found — defaults |
| Cleanup log      | ⚠      | Not yet created — will be created on first clean run |
+------------------+--------+------------------------------------------------------+
All checks passed
```

**JSON mode (`freespace doctor --json`):**
```json
{
  "checks": [
    {"name": "Full Disk Access", "status": "pass", "message": "Granted"},
    {"name": "Protected paths", "status": "pass", "message": "6/6 verified"},
    {"name": "Config file", "status": "warn", "message": "...not found — defaults will be used..."},
    {"name": "Cleanup log", "status": "warn", "message": "Not yet created..."}
  ],
  "overall": "warn"
}
```

## Task Commits

1. **Task 1+2: Implement DoctorCheck types, four check functions, run() with table/JSON/exit-code** - `f997bf2` (feat)

**Plan metadata:** (see docs commit)

## Files Created/Modified

- `freespace/src/commands/doctor.rs` - Full implementation: CheckStatus enum, DoctorCheck struct, 4 check functions, render_doctor_table, run()

## Decisions Made

- TCC probe via `std::fs::metadata` on `~/Library/Safari/History.db` per locked decision in 08-CONTEXT.md
- Used `serde(rename_all = "snake_case")` on CheckStatus so status serializes as `"pass"`, `"fail"`, `"warn"` (lowercase, not `"Pass"`)
- anyhow::bail used for fail exit (never std::process::exit) — ensures stdout/stderr buffers are flushed
- `#[cfg(target_os = "macos")]` guard on protected_paths call with non-macOS warn fallback
- `toml::Value` used in config check (generic parse) rather than `Config` struct — intentional: detects broken TOML syntax while being permissive of unknown keys

## Deviations from Plan

None - plan executed exactly as written. Tasks 1 and 2 were implemented together since they operate on the same file and the combined implementation is cleaner.

## DIAG-01 and DIAG-02 Status: COMPLETE

- **DIAG-01:** `freespace doctor` runs 4 checks, renders comfy_table in human mode and structured JSON in --json mode, exits 0/1 based on failure presence
- **DIAG-02:** Every fail/warn check carries specific actionable remediation (e.g., "Open System Settings > Privacy & Security > Full Disk Access and add freespace"; config file messages include full path and corrective action)

## Phase 8 Complete — Freespace v1 Ready

Phase 8 (doctor-and-polish) is now complete:
- Plan 01: completions subcommand, Wave 0 RED test scaffold
- Plan 02: doctor implementation, all tests GREEN

All Phase 1-8 plans executed. Freespace v1 is feature-complete and ready for `/gsd:verify-work`.

## Issues Encountered

None.

## Next Phase Readiness

- Phase 8 is the final development phase for Freespace v1
- Ready for `/gsd:verify-work` — all commands implemented, all tests passing
- No blockers

---
*Phase: 08-doctor-and-polish*
*Completed: 2026-04-11*
