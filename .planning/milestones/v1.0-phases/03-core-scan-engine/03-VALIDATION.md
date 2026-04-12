---
phase: 3
slug: core-scan-engine
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-29
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml (existing) |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test -- --nocapture` |
| **Estimated runtime** | ~8 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test -- --nocapture`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 3-01-01 | 03-01 | 1 | SCAN-02,SCAN-03,SCAN-04 | unit | `cargo test fs_scan::` | ❌ W0 | ⬜ pending |
| 3-01-02 | 03-01 | 1 | SCAN-01,SCAN-05 | integration | `cargo test --test scan_cmd` | ❌ W0 | ⬜ pending |
| 3-01-03 | 03-01 | 1 | SCAN-01,SCAN-02 | unit | `cargo test analyze::` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/fs_scan/mod.rs` — new module (no existing file)
- [ ] `src/analyze/mod.rs` — new module stub (no existing file)
- [ ] `tests/scan_cmd.rs` — integration tests for scan command

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Scanner doesn't crash on TCC-protected dirs | SCAN-05 | Requires real macOS TCC state | Run `cargo run -- scan ~/Library/Mail` — should complete without panic, show skipped count |
| Physical size matches `du` output | SCAN-04 | Requires filesystem validation | Run `cargo run -- scan /usr/bin` and compare with `du -sk /usr/bin` |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
