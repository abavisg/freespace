# Phase 2: Volume Summary - Context

**Gathered:** 2026-03-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 2 delivers the `freespace summary` command. After this phase: running `freespace summary` shows all mounted volumes with mount point, total bytes, used bytes, and available bytes in a human-readable table. `freespace summary --json` outputs clean JSON on stdout. This is the first command with real disk logic, and it validates the platform::macos module and output module wiring established in Phase 1.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All implementation choices are at Claude's discretion. Key constraints from research:
- Use `sysinfo` crate `Disks` API for volume enumeration — it's already in Cargo.toml
- If `sysinfo` doesn't expose filesystem type strings on macOS, fall back to `statvfs` via `nix` crate or just omit filesystem type
- Output: human-readable `comfy-table` table by default; clean JSON with `--json`
- JSON must go to stdout only; logs/errors to stderr
- The `platform::macos` module already exists — add volume logic there or in `commands/summary.rs`
- `sysinfo` Disks API is blocking — do not call in a tight loop
- Human-readable sizes: use appropriate units (B, KB, MB, GB, TB)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/output/mod.rs` — already has `write_json()` and table output infrastructure from Phase 1
- `src/platform/macos.rs` — platform module already exists with cfg gates
- `src/commands/summary.rs` — stub already exists, needs real implementation
- `Cargo.toml` — `sysinfo = "0.33"` and `comfy-table = "7"` already in deps

### Established Patterns
- `#[cfg(target_os = "macos")]` for platform-specific code
- `anyhow::Result` in command handlers
- `--json` propagated via `Cli::json` global flag
- `eprintln!` for errors/logs, stdout reserved for structured output

### Integration Points
- `src/commands/summary.rs` → `src/output/mod.rs` for table/JSON rendering
- `src/commands/summary.rs` → `src/platform/macos.rs` for volume info

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
