# Phase 8: Doctor and Polish — Research

**Researched:** 2026-04-11
**Domain:** Rust CLI diagnostics, TCC/FDA probing, clap_complete shell completions
**Confidence:** HIGH

## Summary

Phase 8 has two well-bounded implementation tasks: expanding the `doctor.rs` stub into a full self-diagnostic command, and adding a `Completions` subcommand via `clap_complete`. Both domains are well-understood within the project's existing codebase — all required libraries are already in `Cargo.toml` except `clap_complete`, which must be added. The `comfy_table` pattern for dual table+JSON output is already established in `clean.rs` and `summary.rs` and transfers directly to `doctor.rs`.

The TCC probe strategy (attempt `std::fs::metadata` on a known FDA-gated file) is a pure-Rust approach with no subprocess or external dependencies. The three-outcome pattern (pass/fail/warn) maps cleanly to the required check-by-check table with `comfy_table`. Exit code behavior is binary: exit 1 for any `✗` failure, exit 0 for all-pass or warnings-only.

The `clap_complete` integration requires adding the crate as a dependency, adding a `Completions { shell: Shell }` variant to the `Commands` enum in `cli.rs`, dispatching it in `main.rs`, and calling `generate()` to stdout. The `Cli` struct already derives `CommandFactory` (implied by `Parser` derive) so `Cli::command()` is available for passing to `generate()`.

**Primary recommendation:** Implement doctor checks sequentially (TCC probe → protected paths → config file → cleanup log), render with `comfy_table`, emit structured JSON via `write_json`, and exit 1 on any hard failure. Add `clap_complete` dependency and wire `Completions` subcommand to stdout generation.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

1. **Doctor output format:** Check-by-check table via `comfy-table`. Each check is a row: `Check | Status (✓/✗/⚠) | Message`. JSON output is an array of check objects with `name`, `status`, `message` fields plus an `overall` field.

2. **TCC probe strategy:** Attempt `std::fs::metadata(home_dir + "Library/Safari/History.db")`. Three outcomes: `Ok(_)` = pass, `Err(PermissionDenied)` = fail with remediation message "Open System Settings > Privacy & Security > Full Disk Access and add freespace", `Err(_)` other = warn "Cannot determine — Safari History not present".

3. **Exit code behavior:** Binary. Exit 0 = all pass or warnings only. Exit 1 = one or more hard failures (`✗`). Warnings (`⚠`) do not cause non-zero exit.

4. **Shell completions:** `clap_complete` via `freespace completions <shell>` subcommand. Add `Completions { shell: clap_complete::Shell }` variant. Prints script to stdout. Add `clap_complete` to `[dependencies]` in Cargo.toml.

5. **No release tooling for v1** — no cargo-dist, no release scripts.

### Claude's Discretion

- Exact set of doctor checks beyond the 3 required (TCC, protected paths, config) — may add cleanup log existence as a ⚠ warn-only check
- Specific protected path used for TCC probe (Safari History.db preferred; if absent, Claude may try an alternative known TCC-gated path)
- Ordering of checks in the table output

### Deferred Ideas (OUT OF SCOPE)

- Doctor output formats: grouped-by-severity, narrative/prose — as config options
- Alternative TCC probe strategies: `spctl`, `tcc` subprocess, alternative probe paths
- Distinct exit codes for warnings vs hard failures
- Release tooling: cargo-dist, release scripts, binary packaging
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DIAG-01 | `freespace doctor` runs self-diagnostics: TCC/Full Disk Access status, protected-path verification, config file validity | TCC probe via `std::fs::metadata`; `platform::macos::protected_paths()` reused; `config::load_config()` reused; all three map to table rows |
| DIAG-02 | Doctor reports actionable remediation for each detected issue | Each check struct carries a `message` field; fail/warn outcomes provide specific human-readable remediation text |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `comfy-table` | 7.2 | Table rendering for doctor output | Already in Cargo.toml; used by categories, summary, clean commands |
| `serde_json` | 1.0 | JSON output via `write_json()` | Already in Cargo.toml; established pattern in all commands |
| `dirs` | 6.0 | `home_dir()` for TCC probe path + config path construction | Already in Cargo.toml |
| `anyhow` | 1.0 | Error propagation and `bail!` for non-zero exit | Already in Cargo.toml; pattern for all command handlers |
| `clap_complete` | 4.6.1 | Shell completion script generation | Must be added to Cargo.toml |
| `clap` | 4.6 | `CommandFactory` trait via `Parser` derive — needed for `Cli::command()` call | Already in Cargo.toml |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tracing` | 0.1 | Log warnings to stderr only | Warn on inconclusive TCC result, any check errors |
| `chrono` | 0.4 | Timestamp formatting | Available if timestamp is needed in doctor output |

**Installation (only addition needed):**
```bash
cargo add clap_complete@4.6
```

Or manually in `Cargo.toml`:
```toml
clap_complete = "4.6"
```

**Version verified:** `cargo search clap_complete` returns `clap_complete = "4.6.1"` — current as of 2026-04-11.

## Architecture Patterns

### Recommended Structure

No new files needed. Changes are to:
```
freespace/src/
├── commands/
│   └── doctor.rs           # Expand stub → full implementation
├── cli.rs                  # Add Completions { shell: Shell } variant
├── main.rs                 # Add dispatch for Commands::Completions
└── Cargo.toml              # Add clap_complete = "4.6"
```

New integration test:
```
freespace/tests/
└── doctor_cmd.rs           # Integration tests for DIAG-01, DIAG-02
```

### Pattern 1: Doctor Check Struct

The cleanest approach is a `DoctorCheck` struct that carries all data for one row, collected into a `Vec<DoctorCheck>`, then rendered as table or JSON.

```rust
// Source: pattern from freespace/src/commands/clean.rs + CONTEXT.md spec
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
}

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
}
```

### Pattern 2: Table Rendering (established in project)

```rust
// Source: freespace/src/commands/clean.rs render_preview_table pattern
use comfy_table::{Table, presets::UTF8_FULL};

fn render_doctor_table(checks: &[DoctorCheck]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(["Check", "Status", "Message"]);
    for check in checks {
        let symbol = match check.status {
            CheckStatus::Pass => "✓",
            CheckStatus::Fail => "✗",
            CheckStatus::Warn => "⚠",
        };
        table.add_row([&check.name, symbol, &check.message]);
    }
    println!("{table}");
}
```

### Pattern 3: TCC Probe (decided in CONTEXT.md)

```rust
// Source: CONTEXT.md locked decision
fn check_full_disk_access(home: &std::path::Path) -> DoctorCheck {
    let probe = home.join("Library/Safari/History.db");
    match std::fs::metadata(&probe) {
        Ok(_) => DoctorCheck {
            name: "Full Disk Access".into(),
            status: CheckStatus::Pass,
            message: "Granted".into(),
        },
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => DoctorCheck {
            name: "Full Disk Access".into(),
            status: CheckStatus::Fail,
            message: "Open System Settings > Privacy & Security > Full Disk Access and add freespace".into(),
        },
        Err(_) => DoctorCheck {
            name: "Full Disk Access".into(),
            status: CheckStatus::Warn,
            message: "Cannot determine — Safari History not present".into(),
        },
    }
}
```

### Pattern 4: Protected Paths Verification

Reuse existing `platform::macos::protected_paths()` — verify each of the 6 paths resolves (exists via `std::path::Path::exists()`). Report `N/6 verified` or list which paths failed.

```rust
// Source: freespace/src/platform/macos.rs — protected_paths() returns Vec<PathBuf>
fn check_protected_paths() -> DoctorCheck {
    #[cfg(target_os = "macos")]
    {
        let paths = crate::platform::macos::protected_paths();
        let total = paths.len();
        let verified = paths.iter().filter(|p| p.exists()).count();
        if verified == total {
            DoctorCheck {
                name: "Protected paths".into(),
                status: CheckStatus::Pass,
                message: format!("{verified}/{total} verified"),
            }
        } else {
            DoctorCheck {
                name: "Protected paths".into(),
                status: CheckStatus::Warn,
                message: format!("{verified}/{total} verified — some system paths not resolvable"),
            }
        }
    }
}
```

### Pattern 5: Config File Check

```rust
// Source: freespace/src/config/ — config path is home_dir/.config/Freespace/config.toml
fn check_config_file(home: &std::path::Path) -> DoctorCheck {
    let config_path = home.join(".config/Freespace/config.toml");
    if config_path.exists() {
        DoctorCheck {
            name: "Config file".into(),
            status: CheckStatus::Pass,
            message: config_path.display().to_string(),
        }
    } else {
        DoctorCheck {
            name: "Config file".into(),
            status: CheckStatus::Warn,
            message: format!("{} not found (defaults will be used)", config_path.display()),
        }
    }
}
```

### Pattern 6: Cleanup Log Check (Claude's Discretion — warn-only)

```rust
// Source: freespace/src/commands/clean.rs — state_dir pattern
fn check_cleanup_log(home: &std::path::Path) -> DoctorCheck {
    let log_path = home.join(".local/state/Freespace/cleanup.log");
    if log_path.exists() {
        DoctorCheck {
            name: "Cleanup log".into(),
            status: CheckStatus::Pass,
            message: log_path.display().to_string(),
        }
    } else {
        DoctorCheck {
            name: "Cleanup log".into(),
            status: CheckStatus::Warn,
            message: "Not yet created (first run)".into(),
        }
    }
}
```

### Pattern 7: Exit Code Logic

```rust
// Source: CONTEXT.md locked decision — binary exit codes
let has_failure = checks.iter().any(|c| matches!(c.status, CheckStatus::Fail));
if has_failure {
    let count = checks.iter().filter(|c| matches!(c.status, CheckStatus::Fail)).count();
    anyhow::bail!("{count} check(s) failed — see above");
}
Ok(())
```

### Pattern 8: Completions Subcommand

Adding `clap_complete` requires `Cli` to be accessible as a `Command`. With `clap` derive, `Parser` implies `CommandFactory`, so `Cli::command()` is always available.

```rust
// Source: https://github.com/clap-rs/clap/blob/master/clap_complete/examples/completion-derive.rs
// In cli.rs — add to Commands enum:
use clap_complete::Shell;

#[derive(Subcommand)]
pub enum Commands {
    // ... existing variants ...
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

// In main.rs — add dispatch:
Commands::Completions { shell } => {
    use clap::CommandFactory;
    use clap_complete::generate;
    let mut cmd = cli::Cli::command();
    generate(shell, &mut cmd, "freespace", &mut std::io::stdout());
    Ok(())
}
```

### Anti-Patterns to Avoid

- **Calling `protected_paths()` twice:** It already runs in `main.rs` at startup. Doctor should call it directly (the startup call stores to `_protected` which is unused — doctor will call it independently, which is fine for a diagnostic-only command).
- **Writing to stdout in non-JSON mode for intermediate steps:** All table output goes through `println!`, all tracing/warnings to stderr. Keep stdout clean.
- **Using `todo!()` in stub code:** Existing stubs use `eprintln!` not `todo!()` — maintain this pattern.
- **Exiting non-zero for warnings:** Only `CheckStatus::Fail` triggers exit 1. `CheckStatus::Warn` is informational and exits 0.
- **Making the config check a hard failure:** Missing config is not an error (defaults apply) — it is a warn-only check.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Shell completion scripts | Manual script generation | `clap_complete::generate()` | Handles all shell-specific escaping, argument types, subcommand nesting automatically |
| Table formatting | Manual string padding/alignment | `comfy_table` | Already in Cargo.toml; handles terminal width, Unicode, presets |
| JSON serialization | Manual `format!("{}")` | `serde_json` + `write_json()` | Type-safe, handles escaping; established project pattern |
| Home dir resolution | `std::env::var("HOME")` | `dirs::home_dir()` | Handles edge cases (headless sessions, macOS sandbox); already in Cargo.toml |

**Key insight:** The TCC probe is intentionally minimal — `std::fs::metadata` is sufficient and avoids subprocess overhead or platform-specific APIs that could fail in sandboxed environments.

## Common Pitfalls

### Pitfall 1: `Cli::command()` availability for completions generation

**What goes wrong:** `clap_complete::generate()` requires a `&mut Command`, not `&mut Cli`. Developers sometimes pass the wrong type.

**Why it happens:** With clap derive, `Cli` and `Command` are different types — `Cli` is the user-defined struct, `Command` is clap's builder type.

**How to avoid:** Import `clap::CommandFactory` and call `Cli::command()` to get the builder. This is the standard pattern from the official clap_complete example.

**Warning signs:** Compiler error "the trait `Generator` is not implemented" or "expected `Command`, found `Cli`".

### Pitfall 2: TCC probe path absence is not a hard failure

**What goes wrong:** Treating `Err(_)` on the Safari probe as a definitive FDA-denied result. Safari may simply not be installed (rare but possible, e.g., fresh macOS setup, developer machines).

**Why it happens:** Conflating "cannot read file" with "permission denied" — these are distinct `io::ErrorKind` values.

**How to avoid:** Pattern-match specifically on `ErrorKind::PermissionDenied` for the fail case, and use a catch-all `Err(_)` for the warn/inconclusive case (as specified in CONTEXT.md).

### Pitfall 3: Non-zero exit swallowing JSON output

**What goes wrong:** Using `std::process::exit(1)` to exit non-zero causes the process to terminate without flushing buffers, potentially dropping JSON output.

**Why it happens:** Developers conflate OS-level exit with returning an error through `anyhow`.

**How to avoid:** Use `anyhow::bail!()` to return an error from `run()`, let `main()` handle exit code via the `anyhow::Result` return. This ensures stdout is flushed before exit. The existing project pattern already does this correctly.

### Pitfall 4: Table preset inconsistency

**What goes wrong:** Using different `comfy_table` presets across commands, producing inconsistent visual output.

**Why it happens:** `UTF8_FULL` vs `UTF8_BORDERS_ONLY` vs no preset — easy to pick the wrong one.

**How to avoid:** Check existing commands to confirm which preset they use. `clean.rs` uses `Table::new()` without an explicit preset, relying on defaults. Follow the same pattern for visual consistency.

### Pitfall 5: `#[cfg(target_os = "macos")]` guard scope

**What goes wrong:** The `protected_paths()` function is `#[cfg(target_os = "macos")]` — calling it from `doctor.rs` without a matching cfg guard causes a compile error on non-macOS.

**Why it happens:** The guard on `platform::macos` functions only compiles them on macOS. Doctor calling them unchecked breaks Linux/Windows builds (if ever attempted).

**How to avoid:** Wrap the macOS-specific checks in `#[cfg(target_os = "macos")]` blocks within `doctor.rs`, mirroring the pattern in `main.rs` which already does `#[cfg(target_os = "macos")] let _protected = platform::macos::protected_paths();`.

## Code Examples

### Complete `run()` signature and flow (established pattern)

```rust
// Source: freespace/src/commands/doctor.rs stub + CONTEXT.md spec
pub fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve home directory"))?;

    let checks = vec![
        check_full_disk_access(&home),
        check_protected_paths(),
        check_config_file(&home),
        check_cleanup_log(&home),  // warn-only at Claude's discretion
    ];

    let overall = if checks.iter().any(|c| matches!(c.status, CheckStatus::Fail)) {
        "fail"
    } else if checks.iter().any(|c| matches!(c.status, CheckStatus::Warn)) {
        "warn"
    } else {
        "pass"
    };

    if json {
        crate::output::write_json(&serde_json::json!({
            "checks": checks,
            "overall": overall,
        }))?;
    } else {
        render_doctor_table(&checks);
        let fail_count = checks.iter().filter(|c| matches!(c.status, CheckStatus::Fail)).count();
        if fail_count > 0 {
            println!("{fail_count} check(s) failed — see above");
        } else {
            println!("All checks passed");
        }
    }

    // Exit non-zero on any hard failure
    if overall == "fail" {
        let count = checks.iter().filter(|c| matches!(c.status, CheckStatus::Fail)).count();
        anyhow::bail!("{count} check(s) failed");
    }
    Ok(())
}
```

### `clap_complete` generate call (verified from official example)

```rust
// Source: https://github.com/clap-rs/clap/blob/master/clap_complete/examples/completion-derive.rs
// In main.rs dispatch:
Commands::Completions { shell } => {
    use clap::CommandFactory;
    use clap_complete::generate;
    let mut cmd = cli::Cli::command();
    generate(shell, &mut cmd, "freespace", &mut std::io::stdout());
    Ok(())
}
```

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` / Rust toolchain | Building the project | ✓ | cargo 1.89.0, rustc 1.89.0 | — |
| `clap_complete` crate | Completions subcommand | ✓ (via crates.io) | 4.6.1 | — |
| macOS (target_os = "macos") | TCC probe, protected paths check | ✓ (macOS 25.3.0) | Darwin 25.3.0 | Doctor skips macOS-specific checks on other platforms via `#[cfg]` |
| `~/Library/Safari/History.db` | TCC probe | Present on most macOS | — | Fallback to Warn if file absent (CONTEXT.md decision) |

**Missing dependencies with no fallback:** None — all dependencies are available.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in (`#[test]`) + `assert_cmd` 2.0 for integration tests |
| Config file | None (Rust test harness is built-in) |
| Quick run command | `cargo test -p freespace doctor` |
| Full suite command | `cargo test -p freespace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DIAG-01 | `doctor` exits 0 when all checks pass | integration | `cargo test -p freespace doctor_exits_0_all_pass` | ❌ Wave 0 |
| DIAG-01 | `doctor` exits 1 when a hard check fails | integration | `cargo test -p freespace doctor_exits_1_on_failure` | ❌ Wave 0 |
| DIAG-01 | `doctor --json` outputs valid JSON with `checks` array | integration | `cargo test -p freespace doctor_json_structure` | ❌ Wave 0 |
| DIAG-01 | `doctor --json` JSON has `overall` field | integration | `cargo test -p freespace doctor_json_overall_field` | ❌ Wave 0 |
| DIAG-02 | Doctor output contains actionable remediation text | integration | `cargo test -p freespace doctor_remediation_message` | ❌ Wave 0 |
| DIAG-01 | `completions zsh` exits 0 and writes to stdout | integration | `cargo test -p freespace completions_zsh_exits_0` | ❌ Wave 0 |
| DIAG-01 | `completions bash` exits 0 and writes to stdout | integration | `cargo test -p freespace completions_bash_exits_0` | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p freespace doctor`
- **Per wave merge:** `cargo test -p freespace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `freespace/tests/doctor_cmd.rs` — covers DIAG-01, DIAG-02 (doctor table + JSON + exit codes + completions)

*(Existing test infrastructure covers all other commands; only doctor_cmd.rs is new.)*

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Subprocess `tcc` or `spctl` for FDA check | Pure `std::fs::metadata` probe | clap_complete 4.x era | No subprocess, no permission issues checking permissions |
| Manual completion scripts per shell | `clap_complete::generate()` | clap 3.x onward | Single API, all shells, always up to date with CLI structure |
| `clap::Shell` in `clap` crate | `clap_complete::Shell` in separate crate | clap 3.0 | Separated to reduce compile times for apps not needing completions |

**Deprecated/outdated:**
- `clap_generate` (old crate name): replaced by `clap_complete` since clap 3.x — do not use `clap_generate`
- `clap::Shell` (old location): now lives in `clap_complete::Shell` — import path changed

## Open Questions

1. **Safari History.db probe on machines without Safari**
   - What we know: The CONTEXT.md specifies Safari History.db as primary probe; file absent → Warn
   - What's unclear: On machines with no Safari usage but with Safari installed, the `.db` may not exist initially
   - Recommendation: The CONTEXT.md outcome handles this correctly (Warn, not Fail) — no further research needed

2. **comfy_table preset matching existing commands**
   - What we know: `clean.rs` uses `Table::new()` without an explicit preset call
   - What's unclear: Whether other commands use a specific preset for consistent visual style
   - Recommendation: Read one other command (e.g., `summary.rs`) to confirm preset before implementing; match existing style

## Sources

### Primary (HIGH confidence)
- Official clap_complete example — `https://github.com/clap-rs/clap/blob/master/clap_complete/examples/completion-derive.rs` — `CommandFactory`, `generate()`, `Shell` derive pattern verified
- `freespace/src/commands/clean.rs` — table + JSON dual output established pattern (codebase)
- `freespace/src/platform/macos.rs` — `protected_paths()` reusable function (codebase)
- `freespace/src/output/mod.rs` — `write_json()` function (codebase)
- `freespace/Cargo.toml` — confirmed all deps except `clap_complete` already present (codebase)
- `cargo search clap_complete` — verified version 4.6.1 current as of 2026-04-11

### Secondary (MEDIUM confidence)
- `docs.rs/clap_complete` — API overview for `generate()`, `Generator`, `Shell` enum confirmed; Shell variants confirmed: Bash, Elvish, Fish, PowerShell, Zsh

### Tertiary (LOW confidence)
- WebSearch results confirming `clap_complete::Shell` moved from `clap` crate — cross-verified with official example import paths

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all deps confirmed in Cargo.toml; clap_complete version verified via `cargo search`
- Architecture: HIGH — directly modeled on existing codebase patterns (`clean.rs`, `summary.rs`, `main.rs`)
- TCC probe: HIGH — locked decision in CONTEXT.md; pure std library, no external deps
- clap_complete integration: HIGH — verified from official clap repo example
- Pitfalls: HIGH — derived from direct code inspection of project patterns and clap_complete docs

**Research date:** 2026-04-11
**Valid until:** 2026-05-11 (clap_complete is stable; comfy_table is stable; TCC probe strategy is macOS-stable)
