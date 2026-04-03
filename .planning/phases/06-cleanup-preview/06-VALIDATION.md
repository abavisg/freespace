---
phase: 6
slug: cleanup-preview
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-03
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test preview` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test preview`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 6-W0-01 | Wave 0 | 0 | PREV-01, PREV-02, PREV-03 | integration | `cargo test clean_preview_cmd` | ❌ W0 | ⬜ pending |
| 6-01-01 | 01 | 1 | PREV-01 | unit | `cargo test preview` | ✅ | ⬜ pending |
| 6-01-02 | 01 | 1 | PREV-02, PREV-03 | integration | `cargo test clean_preview_cmd` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/clean_preview_cmd.rs` — integration tests for `freespace clean preview` (table output, --json output, idempotency/read-only guarantee, JSON fields)

*Tests may be written inline as part of implementation tasks (TDD pattern).*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| No files modified or deleted after `freespace clean preview` | PREV-02 | Filesystem mutation check requires live environment comparison | Run `freespace clean preview` on a real system; verify mtime of all listed files is unchanged |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
