# Freespace

## What This Is

Freespace is a terminal-first Rust CLI for inspecting, categorising, and safely reclaiming disk space on macOS. It is designed for developers, engineers, and power users who want clear visibility into disk usage, deterministic categorisation, and safe cleanup workflows — all scriptable via JSON output.

## Core Value

A power user can go from zero knowledge to safe, informed disk cleanup in a single session — with no surprises and no accidental deletions.

## Current State

**v1.0 shipped 2026-04-12.** All 8 phases complete, 11 plans executed, 32/32 v1 requirements validated.

The full workflow is live: `summary` → `scan` → `categories` / `largest` → `clean preview` → `clean apply`, plus `hidden`, `caches`, `doctor`, and shell completions. The binary is installed at `~/.cargo/bin/freespace`.

- **2,532 LOC** Rust source (src/) + **1,525 LOC** integration tests (tests/)
- **~104 tests** passing (unit + integration), zero failures
- **19 feature commits** across a 15-day build (2026-03-28 → 2026-04-12)

## Requirements

### Validated (v1.0)

- ✓ CLI skeleton with all subcommands, cfg-gated macOS platform module — v1.0 (Phase 1)
- ✓ Platform isolation, protected-path canonicalization, config system, --json global flag — v1.0 (Phase 1)
- ✓ `freespace summary` — mounted volumes with total/used/available space — v1.0 (Phase 2)
- ✓ `freespace scan` — streaming traversal, hardlink dedup, physical-size accounting, TCC-tolerant — v1.0 (Phase 3)
- ✓ `freespace categories` — 14-category macOS-aware classifier — v1.0 (Phase 4)
- ✓ `freespace hidden` — dotfile listing with sizes — v1.0 (Phase 4)
- ✓ `freespace caches` — known-dir enumeration with safety classification — v1.0 (Phase 4)
- ✓ `freespace largest` — BinaryHeap top-N aggregation, memory-bounded — v1.0 (Phase 5)
- ✓ `freespace clean preview` — read-only gate, safety classification, reclaimable total — v1.0 (Phase 6)
- ✓ `freespace clean apply` — Trash-first, --force guard, protected-path enforcement, audit log — v1.0 (Phase 7)
- ✓ `freespace doctor` — TCC/FDA probe, protected-path check, config/log checks, actionable remediation — v1.0 (Phase 8)
- ✓ `freespace completions` — zsh/bash/fish shell completion scripts — v1.0 (Phase 8)

### Active (v1.1 candidates)

- [ ] Parallel directory traversal via `rayon`/`jwalk` for large volumes
- [ ] Progress indicator for long-running scans
- [ ] Export scan results to file

### Out of Scope

- Exact replication of macOS Storage UI — terminal-native UX is the goal
- Automatic cleanup without preview — violates Inspect→Classify→Preview→Clean safety order
- Deep system-level integrations (Photos internals, Mail internals) — too risky for MVP
- AI-driven cleanup suggestions — explicitly deferred to future, opt-in only, advisory only, never auto-delete
- GUI or web interface — terminal-first, scriptable
- Interactive TUI (ncurses-style) — conflicts with JSON-first scripting philosophy

## Context

- **Language:** Rust (edition 2021)
- **Target platform:** macOS only (platform::macos module for macOS-specific behavior)
- **Key crates:** clap 4.6 (CLI), walkdir 2.5 (traversal), serde + toml (config/JSON), trash 5.2 (safe deletion), comfy-table 7.2 (output), sysinfo 0.38 (volumes), clap_complete 4.6 (shell completions)
- **Safety constraint:** Inspect→Classify→Preview→Clean order is architectural, not advisory — cleanup cannot run without a prior scan session
- **Protected paths (immutable):** /System, /usr, /bin, /sbin, /private — canonicalized at startup
- **Cleanup log:** ~/.local/state/Freespace/cleanup.log
- **Config:** ~/.config/Freespace/config.toml

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust as implementation language | Performance, safety guarantees, memory efficiency for large directory traversal | ✓ Good — zero memory issues, fast even on large dirs |
| Trash-first deletion model | Prevents accidental data loss; permanent delete requires explicit --force flag | ✓ Good — safe default validated in Phase 7 |
| Path-first classification priority | macOS known paths (~/Library/Caches, ~/.ollama) more reliable than extension guessing | ✓ Good — no misclassification issues in testing |
| macOS-only v1 | Avoids platform abstraction complexity; platform module isolates this for future expansion | ✓ Good — clean cfg-gated module boundaries |
| Build order enforced (no cleanup before scan) | Safety-critical: scan and classification must be reliable before cleanup is usable | ✓ Good — session file pattern works cleanly |
| TCC probe via Safari/History.db metadata | Avoids NSWorkspace entitlement complexity, deterministic pass/fail/warn semantics | ✓ Good — works reliably across FDA states |
| clap_complete for shell completions | Drop-in with clap 4.6, all 3 shells in one dep | ✓ Good — zsh/bash/fish all emit valid scripts |
| anyhow::bail for non-zero exit in doctor | Preserves stdout/stderr buffer flushing vs std::process::exit | ✓ Good — flush semantics correct |

## Constraints

- **Safety:** Cleanup before scan/classification is explicitly forbidden — hard rule, not suggestion
- **Platform:** macOS only for v1 — platform module isolates OS-specific behavior
- **Deletion:** Trash preferred over permanent delete; --force required for permanent; blocked paths are immutable
- **Output:** JSON must be clean on stdout only; logs/errors go to stderr
- **Performance:** Must handle large directories (100k+ files) via streaming aggregation

---
*Last updated: 2026-04-12 after v1.0 milestone*
