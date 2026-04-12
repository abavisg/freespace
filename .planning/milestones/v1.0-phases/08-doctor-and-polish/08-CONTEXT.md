# Phase 8: Doctor and Polish — Context

**Gathered:** 2026-04-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 8 delivers two things:

1. **`freespace doctor`** — self-diagnostics that check TCC/Full Disk Access status, protected-path verification, and config file validity. Reports results in a check-by-check table with actionable remediation. Exits non-zero when any check fails.

2. **Shell completions** — `freespace completions <shell>` subcommand generating zsh/bash/fish completion scripts via `clap_complete`.

No release tooling, no cargo-dist, no additional output formats for v1. Doctor and completions only.

</domain>

<decisions>
## Implementation Decisions

### 1. Doctor Output Format

**Decision: Check-by-check table**

Each check is a row: `Check | Status (✓/✗/⚠) | Message`. Renders via `comfy-table` (already in Cargo.toml). JSON output mirrors the same structure — an array of check objects.

Example table:
```
Check                    Status  Message
───────────────────────────────────────────────────────
Full Disk Access         ✓       Granted
Protected paths          ✓       6/6 verified
Config file              ✓       ~/.config/Freespace/config.toml
Cleanup log              ⚠       Not yet created (first run)
```

JSON shape:
```json
{
  "checks": [
    { "name": "Full Disk Access", "status": "pass", "message": "Granted" },
    { "name": "Protected paths", "status": "pass", "message": "6/6 verified" },
    { "name": "Config file",     "status": "pass", "message": "~/.config/Freespace/config.toml" },
    { "name": "Cleanup log",     "status": "warn", "message": "Not yet created (first run)" }
  ],
  "overall": "pass"
}
```

**Deferred to v2:** grouped-by-severity format, narrative format — as config options.

---

### 2. TCC Probe Strategy

**Decision: Attempt-read a known TCC-gated file**

Try `std::fs::metadata(home_dir + "Library/Safari/History.db")`:
- `Ok(_)` → FDA granted → Pass
- `Err(PermissionDenied)` → FDA not granted → Fail, with remediation message: "Open System Settings > Privacy & Security > Full Disk Access and add freespace"
- `Err(_)` (file absent or other) → Inconclusive → Warn, message: "Cannot determine — Safari History not present"

Pure Rust, no subprocess, no external dependencies.

**Deferred to v2:** alternative probe paths, `spctl`/`tcc` subprocess strategies — as config options.

---

### 3. Exit Code Behavior

**Decision: Binary exit codes**

- Exit 0 = all checks pass (or warnings only)
- Exit 1 = one or more checks are hard failures

Warnings (⚠) do NOT cause a non-zero exit — they're informational. Only ✗ failures trigger exit 1.

This enables clean scripting: `freespace doctor && freespace clean apply`

**Deferred to v2:** distinct exit codes for warnings vs failures.

---

### 4. Shell Completions

**Decision: `clap_complete` via `freespace completions <shell>` subcommand**

Add a `Completions { shell: clap_complete::Shell }` variant to the CLI. Running it prints the completion script to stdout, which the user redirects to their shell's completions directory.

Supported shells: zsh, bash, fish (whatever `clap_complete::Shell` supports).

```
$ freespace completions zsh > ~/.zsh/completions/_freespace
$ freespace completions bash > /etc/bash_completion.d/freespace
$ freespace completions fish > ~/.config/fish/completions/freespace.fish
```

Add `clap_complete` to `[dependencies]` in Cargo.toml.

**No release tooling for v1** — no cargo-dist, no release scripts.

---

### Claude's Discretion

- Exact set of doctor checks beyond the 3 required (TCC, protected paths, config) — Claude may add cleanup log existence as a ⚠ warn-only check
- Specific protected path used for TCC probe (Safari History.db is preferred; if absent, Claude may try an alternative known TCC-gated path)
- Ordering of checks in the table output

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements
- `.planning/REQUIREMENTS.md` — DIAG-01, DIAG-02 (the only v1 requirements for this phase)
- `.planning/ROADMAP.md` — Phase 8 success criteria (3 items: doctor reports, actionable remediation, non-zero exit)

### Existing code
- `freespace/src/commands/doctor.rs` — current stub to expand
- `freespace/src/platform/macos.rs` — `protected_paths()` reusable for verification check
- `freespace/src/output/mod.rs` — `write_json()` for JSON output
- `freespace/src/cli.rs` — CLI enum to extend with `Completions` variant
- `freespace/src/main.rs` — dispatch to extend with `Commands::Completions`
- `freespace/Cargo.toml` — add `clap_complete` dependency

### Established patterns
- `freespace/src/commands/clean.rs` — pattern for table + JSON dual output
- `freespace/src/commands/summary.rs` — simple command pattern with `comfy_table`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/commands/doctor.rs` — stub with correct signature `run(config: &Config, json: bool) -> anyhow::Result<()>`
- `src/platform/macos.rs` — `protected_paths()` returns `Vec<PathBuf>`; can be called in doctor to verify all 6 paths resolve correctly
- `src/output/mod.rs` — `write_json()` for JSON; `OutputFormat` enum for branching
- `comfy-table` v7.2 — already in Cargo.toml, used by categories/summary/caches commands
- `dirs` v6.0 — already in Cargo.toml, use `dirs::home_dir()` for TCC probe path construction
- `chrono` v0.4 — already in Cargo.toml, available for any timestamp needs

### Established Patterns
- All command handlers: `fn run(config: &Config, json: bool) -> anyhow::Result<()>`
- Table output via `comfy_table::Table` with `load_preset(UTF8_FULL)` or similar
- JSON output via `crate::output::write_json(&value)`
- Errors/logs → stderr via `tracing`; stdout clean
- Non-zero exit via `anyhow::bail!("message")`

### Integration Points
- `Commands::Doctor` already dispatches to `commands::doctor::run()` in `main.rs`
- `Commands::Completions` needs to be added to `cli.rs` and `main.rs`
- `clap_complete` generation needs access to the `Cli` struct — must be done in `main.rs` or a dedicated completions handler

</code_context>

<specifics>
## Specific Ideas

- TCC probe: `~/Library/Safari/History.db` as primary probe path
- Doctor summary line after table: "All checks passed" (exit 0) or "N check(s) failed — see above" (exit 1)
- Completions subcommand prints to stdout (user pipes/redirects it themselves)

</specifics>

<deferred>
## Deferred Ideas

The following were surfaced but explicitly deferred to v2:

- **Doctor output formats**: grouped-by-severity, narrative/prose — as config options
- **Alternative TCC probe strategies**: `spctl`, `tcc` subprocess, alternative probe paths — as config options
- **Distinct exit codes**: separate exit codes for warnings vs hard failures (currently: warnings = exit 0)
- **Release tooling**: `cargo-dist`, release scripts, binary packaging — not needed for v1

</deferred>

---

*Phase: 08-doctor-and-polish*
*Context gathered: 2026-04-11*
