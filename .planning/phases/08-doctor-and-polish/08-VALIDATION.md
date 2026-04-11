---
phase: 8
slug: doctor-and-polish
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-11
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | `freespace/Cargo.toml` |
| **Quick run command** | `cargo test -p freespace doctor` |
| **Full suite command** | `cargo test -p freespace` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p freespace doctor`
- **After every plan wave:** Run `cargo test -p freespace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 8-01-01 | 01 | 0 | DIAG-01 | integration | `cargo test -p freespace doctor_exits_0_all_pass` | ❌ Wave 0 | ⬜ pending |
| 8-01-02 | 01 | 0 | DIAG-01 | integration | `cargo test -p freespace doctor_exits_1_on_failure` | ❌ Wave 0 | ⬜ pending |
| 8-01-03 | 01 | 0 | DIAG-01 | integration | `cargo test -p freespace doctor_json_structure` | ❌ Wave 0 | ⬜ pending |
| 8-01-04 | 01 | 0 | DIAG-01 | integration | `cargo test -p freespace doctor_json_overall_field` | ❌ Wave 0 | ⬜ pending |
| 8-01-05 | 01 | 0 | DIAG-02 | integration | `cargo test -p freespace doctor_remediation_message` | ❌ Wave 0 | ⬜ pending |
| 8-02-01 | 02 | 0 | DIAG-01 | integration | `cargo test -p freespace completions_zsh_exits_0` | ❌ Wave 0 | ⬜ pending |
| 8-02-02 | 02 | 0 | DIAG-01 | integration | `cargo test -p freespace completions_bash_exits_0` | ❌ Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `freespace/tests/doctor_cmd.rs` — integration test stubs for DIAG-01, DIAG-02
- [ ] Tests cover: exit 0 all-pass, exit 1 on failure, JSON shape (`checks` array + `overall`), remediation text, completions zsh/bash exits 0

*Existing `cargo test` infrastructure covers all phase requirements — only new test file needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| TCC/Full Disk Access detection | DIAG-01 | Requires real macOS FDA grant/revoke — cannot mock in CI | Run `freespace doctor` with FDA granted and revoked; verify ✓/✗ status row |
| Doctor table renders correctly | DIAG-01 | Visual terminal rendering | Run `freespace doctor` in terminal; inspect table alignment |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
