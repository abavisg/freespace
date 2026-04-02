---
phase: 5
slug: analysis-layer-and-largest-files
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-02
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml |
| **Quick run command** | `cargo test largest` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test largest`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 5-W0-01 | Wave 0 | 0 | SCAN-06 | integration | `cargo test largest_cmd` | ❌ W0 | ⬜ pending |
| 5-W0-02 | Wave 0 | 0 | SCAN-06 | unit | `cargo test bounded_heap` | ❌ W0 | ⬜ pending |
| 5-01-01 | 01 | 1 | SCAN-06 | unit | `cargo test bounded_heap` | ✅ | ⬜ pending |
| 5-01-02 | 01 | 1 | SCAN-06 | integration | `cargo test largest_cmd` | ✅ | ⬜ pending |
| 5-02-01 | 02 | 2 | SCAN-06 | integration | `cargo test largest_cmd -- --include-ignored` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/largest_cmd.rs` — 4 integration tests for `freespace largest` command (table output, --json output, --top-n flag, error on invalid path)
- [ ] `src/analysis/mod.rs` or `src/fs_scan/analysis.rs` — unit test stubs for bounded BinaryHeap logic

*These must exist and compile (can be `#[ignore]`) before Wave 1 begins.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Memory stays bounded on very large directories (100k+ files) | SCAN-06 | Heap size validation requires runtime memory profiling | Run `freespace largest /usr --top 20` on a large directory, verify no OOM |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
