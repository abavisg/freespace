---
phase: 1
slug: foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-28
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml (none yet — Wave 0 creates project) |
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
| 1-01-01 | 01 | 0 | FOUND-01 | build | `cargo build` | ❌ W0 | ⬜ pending |
| 1-01-02 | 01 | 1 | FOUND-01 | integration | `cargo test test_help_output` | ❌ W0 | ⬜ pending |
| 1-01-03 | 01 | 1 | FOUND-02 | unit | `cargo test test_platform_module` | ❌ W0 | ⬜ pending |
| 1-01-04 | 01 | 1 | FOUND-03 | unit | `cargo test test_protected_paths` | ❌ W0 | ⬜ pending |
| 1-01-05 | 01 | 1 | FOUND-04 | unit | `cargo test test_config_load` | ❌ W0 | ⬜ pending |
| 1-01-06 | 01 | 1 | FOUND-05 | integration | `cargo test test_stderr_only` | ❌ W0 | ⬜ pending |
| 1-01-07 | 01 | 1 | FOUND-06 | integration | `cargo test test_json_flag` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `Cargo.toml` — project scaffold with all dependencies
- [ ] `src/main.rs` — entry point
- [ ] `src/cli.rs` — clap derive structs
- [ ] `src/commands/mod.rs` — command routing stubs
- [ ] `src/config/mod.rs` — config module stub
- [ ] `src/output/mod.rs` — output module stub
- [ ] `src/platform/mod.rs` — platform module stub
- [ ] `tests/` directory with integration test stubs

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `freespace --help` shows all subcommands in terminal | FOUND-01 | Visual output verification | Run `cargo run -- --help` and verify all subcommands listed |
| Config missing handled gracefully | FOUND-04 | Requires filesystem state | Delete `~/.config/Freespace/config.toml` if exists, run any command, verify no crash |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
