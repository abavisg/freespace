# Freespace

## What This Is

Freespace is a terminal-first Rust CLI for inspecting, categorising, and safely reclaiming disk space on macOS. It is designed for developers, engineers, and power users who want clear visibility into disk usage, deterministic categorisation, and safe cleanup workflows — all scriptable via JSON output.

## Core Value

A power user can go from zero knowledge to safe, informed disk cleanup in a single session — with no surprises and no accidental deletions.

## Requirements

### Validated

- [x] User can view mounted volumes with total, used, and available space — Validated in Phase 2: Volume Summary
- [x] User can scan any path and see total size, file count, directory count — Validated in Phase 3: Core Scan Engine
- [x] User can see disk usage grouped by category (video, audio, images, documents, archives, applications, developer, caches, mail, containers, cloud-sync, hidden, system-related, unknown) — Validated in Phase 4: Classification and Category Commands
- [x] User can inspect hidden files and directories under any path — Validated in Phase 4: Classification and Category Commands
- [x] User can view cache directories with reclaimable size and safety classification — Validated in Phase 4: Classification and Category Commands

### Active
- [ ] User can preview what a cleanup would affect before anything is deleted
- [ ] User can apply cleanup with Trash as default, permanent delete requiring --force, and protected paths blocked
- [ ] All major commands support --json output (clean JSON on stdout, logs on stderr)
- [ ] User can configure exclusions and safe categories via ~/.config/Freespace/config.toml
- [ ] Tool handles permission errors, broken symlinks, and mid-scan deletions without crashing

### Out of Scope

- Exact replication of macOS Storage UI — not the goal; terminal-native UX is
- Automatic cleanup without preview — violates the Inspect→Classify→Preview→Clean safety order
- Deep system-level integrations (Photos internals, Mail internals) — too risky, out of scope for MVP
- AI-driven cleanup suggestions — explicitly deferred to future, opt-in only, advisory only, never auto-delete
- GUI or web interface — terminal-first, scriptable

## Context

- Language: Rust
- Target platform: macOS only (platform::macos module for macOS-specific behavior)
- Key crates: clap (CLI), walkdir (traversal), serde + toml (config/JSON), trash (safe deletion), comfy-table (output)
- Safety is the defining constraint: the tool enforces Inspect→Classify→Preview→Clean order; cleanup cannot run before scan and classification are reliable
- Protected paths that can never be deleted: /System, /usr, /bin, /sbin, /private
- Cleanup actions logged to ~/.local/state/Freespace/cleanup.log
- Config stored at ~/.config/Freespace/config.toml
- Performance: streaming aggregation, no loading entire directory trees into memory

## Constraints

- **Safety**: Cleanup before scan/classification is explicitly forbidden — not a suggestion, a hard rule
- **Platform**: macOS only for v1 — platform module isolates OS-specific behavior
- **Deletion**: Trash preferred over permanent delete; --force required for permanent; blocked paths are immutable
- **Output**: JSON must be clean on stdout only; logs/errors go to stderr
- **Performance**: Must handle large directories (100k+ files) via streaming aggregation

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust as implementation language | Performance, safety guarantees, memory efficiency for large directory traversal | — Pending |
| Trash-first deletion model | Prevents accidental data loss; permanent delete requires explicit --force flag | — Pending |
| Path-first classification priority | macOS known paths (~/Library/Caches, ~/.ollama) more reliable than extension guessing | — Pending |
| macOS-only v1 | Avoids platform abstraction complexity; platform module isolates this for future expansion | — Pending |
| Build order enforced (no cleanup before scan) | Safety-critical: scan and classification must be reliable before cleanup is usable | — Pending |

---
*Last updated: 2026-03-28 after initialization*
