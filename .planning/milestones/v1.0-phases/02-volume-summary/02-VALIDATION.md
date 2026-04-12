---
phase: 2
slug: volume-summary
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-29
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml (existing) |
| **Quick run command** | `cargo test` |
| **Full suite command** | `cargo test -- --nocapture` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test`
- **After every plan wave:** Run `cargo test -- --nocapture`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 2-01-01 | 02-01 | 1 | SUMM-01 | unit | `cargo test summary::` | ❌ W0 | ⬜ pending |
| 2-01-02 | 02-01 | 1 | SUMM-02 | unit | `cargo test summary::` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/platform/macos.rs` — add `VolumeInfo` struct and `list_volumes()` function
- [ ] `tests/` — integration test stubs for summary command

*Note: No separate Wave 0 plan needed — VolumeInfo and list_volumes created as first task in Wave 1.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `freespace summary` shows real mounted volumes | SUMM-01 | Requires real macOS disk access | Run `cargo run -- summary` and verify at least `/` volume appears with non-zero sizes |
| `freespace summary --json` produces clean stdout | SUMM-02 | Requires real disk access | Run `cargo run -- summary --json 2>/dev/null` and verify valid JSON array output |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
