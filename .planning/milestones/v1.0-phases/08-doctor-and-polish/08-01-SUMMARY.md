---
plan: 08-01
phase: 08-doctor-and-polish
status: complete
completed: 2026-04-12
---

## Summary

Plan 01 complete: Completions subcommand wired end-to-end, Wave 0 test scaffold in place.

## What Was Built

- **Completions subcommand** — `freespace completions <shell>` works for zsh, bash, fish. Wired via `Commands::Completions { shell: Shell }` variant in `cli.rs` and dispatch arm in `main.rs` using `clap_complete::generate`.
- **Wave 0 test scaffold** — `freespace/tests/doctor_cmd.rs` created with 10 integration tests covering DIAG-01 and DIAG-02.

## RED/GREEN Split

| Test Group | Count | Status |
|-----------|-------|--------|
| completions_* | 3 | GREEN ✓ |
| doctor_* | 6 | RED (expected) |

Doctor tests fail against the existing stub (`{"status":"not_implemented","command":"doctor"}` — no `checks` or `overall` field). This is correct Wave 0 behavior.

## Files Modified

| File | Change |
|------|--------|
| `freespace/Cargo.toml` | `clap_complete = "4.6"` already present (pre-existing) |
| `freespace/src/cli.rs` | Added `completions_subcommand_parses_zsh` and `completions_subcommand_parses_bash` unit tests |
| `freespace/src/main.rs` | Added `Commands::Completions { shell }` match arm using `clap_complete::generate` |
| `freespace/tests/doctor_cmd.rs` | Created — 10 tests (all required test names present) |

## Handoff Note for Plan 02

`doctor.rs` stub is untouched. The 6 doctor_* tests are RED:
- `doctor_exits_0_all_pass` — panics: no `overall` field in stub JSON
- `doctor_exits_1_on_failure` — panics: no `overall` field
- `doctor_json_structure` — panics: no `checks` field
- `doctor_json_overall_field` — panics: no `overall` field
- `doctor_includes_required_checks` — panics: no `checks` field
- `doctor_remediation_message` — panics: no `checks` field

Plan 02 must implement `freespace/src/commands/doctor.rs` to turn all 6 GREEN.
