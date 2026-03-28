# Phase 1: Foundation - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 1 delivers the complete CLI skeleton, safety scaffolding, config system, and error routing infrastructure. After this phase: every subcommand exists and routes correctly, protected paths are canonicalized at startup, --json is wired globally, all errors go to stderr, and the config file is read without crashing. No real disk logic yet — pure infrastructure that everything else builds on.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All implementation choices are at Claude's discretion — pure infrastructure phase. Key constraints from the PRD:
- Use clap 4.6 with derive API (#[derive(Parser, Subcommand)])
- thiserror for domain errors, anyhow in command handlers
- --json flag wired globally; JSON on stdout only, logs/errors on stderr
- Protected paths: /System, /usr, /bin, /sbin, /private — resolved via canonicalize()
- Config at ~/.config/Freespace/config.toml; missing file handled gracefully
- platform::macos module isolated behind #[cfg(target_os = "macos")]
- Project structure: src/main.rs, src/cli.rs, src/commands/, src/fs/, src/classify/, src/analyze/, src/cleanup/, src/config/, src/output/, src/platform/

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- None yet — greenfield project

### Established Patterns
- Rust 2021 edition
- Cargo workspace at project root
- clap derive API for all CLI parsing

### Integration Points
- All downstream phases plug into commands/ module
- output/ module used by every command for table + JSON rendering

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. Safety scaffolding (protected paths, canonicalize) must be in place from day one per research PITFALLS.md.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
