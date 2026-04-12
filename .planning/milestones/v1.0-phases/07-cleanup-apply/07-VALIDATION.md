---
phase: 7
slug: cleanup-apply
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-11
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust integration tests via assert_cmd) |
| **Config file** | freespace/Cargo.toml — `[dev-dependencies]` tempfile, assert_cmd |
| **Quick run command** | `cd freespace && cargo test --test clean_apply_cmd 2>&1` |
| **Full suite command** | `cd freespace && cargo test 2>&1` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd freespace && cargo test --test clean_apply_cmd 2>&1`
- **After every plan wave:** Run `cd freespace && cargo test 2>&1`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 7-01-01 | 01 | 0 | APPLY-01/05 | integration | `cargo test --test clean_apply_cmd test_apply_no_session_fails` | ❌ W0 | ⬜ pending |
| 7-01-02 | 01 | 0 | APPLY-05 | integration | `cargo test --test clean_apply_cmd test_apply_expired_session_fails` | ❌ W0 | ⬜ pending |
| 7-01-03 | 01 | 0 | APPLY-03 | integration | `cargo test --test clean_apply_cmd test_apply_protected_path_never_deletes` | ❌ W0 | ⬜ pending |
| 7-01-04 | 01 | 1 | APPLY-05 | integration | `cargo test --test clean_apply_cmd test_apply_no_session_fails` | ✅ W0 | ⬜ pending |
| 7-01-05 | 01 | 1 | APPLY-01 | integration | `cargo test --test clean_apply_cmd test_apply_trashes_safe_candidates` | ✅ W0 | ⬜ pending |
| 7-01-06 | 01 | 1 | APPLY-02 | integration | `cargo test --test clean_apply_cmd test_apply_force_required_for_permanent_delete` | ✅ W0 | ⬜ pending |
| 7-01-07 | 01 | 1 | APPLY-03 | integration | `cargo test --test clean_apply_cmd test_apply_protected_path_never_deletes` | ✅ W0 | ⬜ pending |
| 7-01-08 | 01 | 1 | APPLY-04 | integration | `cargo test --test clean_apply_cmd test_apply_audit_log_written` | ✅ W0 | ⬜ pending |
| 7-01-09 | 01 | 2 | APPLY-01..05 | integration | `cargo test --test clean_apply_cmd` | ✅ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `freespace/tests/clean_apply_cmd.rs` — stub test file with failing tests for all 5 APPLY requirements
  - `test_apply_no_session_fails` — APPLY-05: no session file → non-zero exit + informative error message
  - `test_apply_expired_session_fails` — APPLY-05: session file older than 1h → non-zero exit
  - `test_apply_protected_path_never_deletes` — APPLY-03: protected path in session → blocked, not deleted
  - `test_apply_trashes_safe_candidates` — APPLY-01: safe candidates moved to trash (or skipped in CI)
  - `test_apply_force_required_for_permanent_delete` — APPLY-02: without --force, permanent delete does not execute
  - `test_apply_audit_log_written` — APPLY-04: after apply, cleanup.log contains JSON entry with timestamp/path/size/action
  - `test_apply_network_volume_warned_and_skipped` — network volume detection (sysinfo integration)
  - `test_apply_json_output` — --json flag produces clean JSON on stdout

*All Wave 0 tests start as `#[ignore]` or `todo!()` stubs — they must compile but can fail. Implementation in Wave 1 makes them pass.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Files recoverable from Finder Trash after `clean apply` | APPLY-01 | Requires real Trash interaction; trash-rs Finder method can't be mocked | Run `freespace clean apply` against a temp dir; open Finder Trash and verify files appear with "Put Back" option |
| y/N confirmation prompt displays before deletion | APPLY-02 gate | Interactive stdin; assert_cmd stdin simulation is fragile | Run `freespace clean apply` in terminal; confirm prompt appears and Enter (default N) aborts |

*If none: "All phase behaviors have automated verification."*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
