---
phase: 4
slug: classification-and-category-commands
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-30
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml (existing) |
| **Quick run command** | `cargo test --lib 2>&1 | tail -5` |
| **Full suite command** | `cargo test 2>&1 | tail -20` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --lib 2>&1 | tail -5`
- **After every plan wave:** Run `cargo test 2>&1 | tail -20`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 4-01-01 | 01 | 1 | CAT-01, CAT-02 | unit | `cargo test classifier` | ❌ W0 | ⬜ pending |
| 4-01-02 | 01 | 1 | CAT-03 | unit | `cargo test categories_cmd` | ❌ W0 | ⬜ pending |
| 4-01-03 | 01 | 2 | HIDD-01, HIDD-02 | unit | `cargo test hidden_cmd` | ❌ W0 | ⬜ pending |
| 4-01-04 | 01 | 2 | CACH-01, CACH-02, CACH-03 | unit | `cargo test caches_cmd` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/classifier/mod.rs` — classifier module stubs for CAT-01, CAT-02
- [ ] `src/classifier/extensions.rs` — extension-to-category mapping
- [ ] `src/classifier/known_dirs.rs` — macOS known-dirs registry
- [ ] `tests/` — integration test stubs for categories/hidden/caches commands

*All test infrastructure uses existing cargo test framework — no additional installs needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Safety classification display (safe/caution/dangerous/blocked) | CACH-03 | Visual table output | Run `freespace caches`, verify each cache row shows a safety label |
| macOS known-dirs coverage accuracy | CAT-02 | Platform-specific, needs real macOS paths | Run `freespace categories ~`, verify ~/Library/Caches → caches, ~/.ollama → developer |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
