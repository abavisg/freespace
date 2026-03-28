# Phase 1: Foundation - Research

**Researched:** 2026-03-28
**Domain:** Rust CLI skeleton — clap 4.6 derive API, config loading, protected paths, stderr logging, platform module isolation
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

All implementation choices are at Claude's discretion — pure infrastructure phase. Key constraints from the PRD:
- Use clap 4.6 with derive API (`#[derive(Parser, Subcommand)]`)
- thiserror for domain errors, anyhow in command handlers
- `--json` flag wired globally; JSON on stdout only, logs/errors on stderr
- Protected paths: /System, /usr, /bin, /sbin, /private — resolved via `canonicalize()`
- Config at `~/.config/Freespace/config.toml`; missing file handled gracefully
- `platform::macos` module isolated behind `#[cfg(target_os = "macos")]`
- Project structure: src/main.rs, src/cli.rs, src/commands/, src/fs/, src/classify/, src/analyze/, src/cleanup/, src/config/, src/output/, src/platform/

### Claude's Discretion

All implementation choices not listed above.

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FOUND-01 | CLI skeleton with all subcommands routed via clap derive API (summary, scan, largest, categories, hidden, caches, clean preview, clean apply, config, doctor) | clap 4.6 derive patterns with `#[derive(Parser, Subcommand)]`; `clean` is a nested subcommand requiring a sub-enum |
| FOUND-02 | Platform module (`platform::macos`) isolates all macOS-specific behavior behind `#[cfg(target_os = "macos")]` | cfg gate patterns and platform trait approach documented in Architecture patterns |
| FOUND-03 | Protected-path constants (/System, /usr, /bin, /sbin, /private) resolved via `canonicalize()` at startup | `std::fs::canonicalize` (realpath on Unix) resolves symlinks; must be called once at startup and stored as absolute PathBufs |
| FOUND-04 | Config system reads `~/.config/Freespace/config.toml` with `[scan] exclude` and `[cleanup] safe_categories` support | `dirs::config_dir()` + `toml` + serde derive; missing file returns `Ok(Config::default())` not an error |
| FOUND-05 | Error handling uses thiserror for domain errors and anyhow in command handlers; all logs/errors go to stderr only | `tracing-subscriber` 0.3.23 with `.with_writer(io::stderr)` replaces env_logger entirely |
| FOUND-06 | `--json` flag wired globally — all commands support it; JSON output is clean stdout only | `#[arg(long, global = true)]` on `json: bool` field in `Cli` struct; `serde_json::to_writer(stdout, &result)` in output module |
</phase_requirements>

---

## Summary

Phase 1 builds the complete Rust CLI infrastructure on which all later phases depend. The work falls into five independent concerns: (1) the clap derive CLI skeleton with all subcommands and a global `--json` flag, (2) the platform module gated behind `#[cfg(target_os = "macos")]`, (3) canonicalized protected-path constants resolved at startup, (4) the TOML config loader with graceful missing-file handling, and (5) stderr-only logging via `tracing-subscriber`.

Every downstream phase plugs into structures created here. The `commands/` module stubs produced in this phase become the entry points for Phases 2-8. The `output` module's stdout/stderr discipline must be established now because it is impossible to retrofit cleanly once command handlers start using `println!` directly.

The key implementation risk is the `clean` subcommand, which has its own sub-subcommands (`preview`/`apply`). clap 4.6 handles this naturally via a nested `CleanCommands` enum inside the `Commands::Clean` variant, but the nesting must be set up correctly from the start.

**Primary recommendation:** Wire the complete CLI skeleton with stub handlers first, verify `--help` output and `--json` propagation, then layer in config loading and protected-path resolution. Logging setup is trivial once the rest compiles.

---

## Standard Stack

### Phase 1 Dependencies Only

The full stack is documented in `.planning/research/STACK.md`. This phase uses a subset:

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| clap | 4.6 | CLI parsing, subcommands, help | Derive API produces compile-time validated CLI with zero boilerplate |
| serde | 1.0.228 | Serialization framework | Required by both `serde_json` and `toml` |
| serde_json | 1.0.149 | JSON stdout output | `to_writer(stdout, &result)` pattern |
| toml | 1.1.0 | Config file deserialization | Deserializes directly into serde-annotated struct; no intermediate step |
| thiserror | 2.0.18 | Typed domain error enums | Domain layers define typed errors callers can match on |
| anyhow | 1.0.102 | Error propagation in main + handlers | Wraps any `std::error::Error` for user-facing display |
| dirs | 6.0.0 | Platform path resolution | `dirs::config_dir()` → `~/Library/Application Support` on macOS |
| tracing | 0.1.44 | Structured logging facade | Used across all modules for `warn!`, `debug!`, `error!` |
| tracing-subscriber | 0.3.23 | Logging subscriber bound to stderr | `fmt().with_writer(io::stderr).init()` replaces env_logger |

**Cargo.toml for this phase:**
```toml
[package]
name = "freespace"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4.6", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "1.1"
thiserror = "2.0"
anyhow = "1.0"
dirs = "6.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Later phases add: walkdir, trash, comfy-table, sysinfo, nix, bytesize, etc.
```

**Version verification:** All versions confirmed against crates.io API on 2026-03-28. tracing 0.1.44 and tracing-subscriber 0.3.23 are the current stable releases.

---

## Architecture Patterns

### Recommended Project Structure (Phase 1 scope)

```
src/
├── main.rs              # parse CLI, init logging, load config, dispatch
├── cli.rs               # Cli struct + Commands enum (clap derive)
├── commands/
│   ├── mod.rs           # pub use all handlers
│   ├── summary.rs       # stub: todo!()
│   ├── scan.rs          # stub: todo!()
│   ├── categories.rs    # stub: todo!()
│   ├── hidden.rs        # stub: todo!()
│   ├── caches.rs        # stub: todo!()
│   ├── clean.rs         # stub: preview + apply sub-dispatch
│   ├── largest.rs       # stub: todo!()
│   ├── config.rs        # stub: todo!()
│   └── doctor.rs        # stub: minimal (verifies protected paths + config read)
├── config/
│   ├── mod.rs           # load_config() -> Result<Config>
│   └── schema.rs        # Config struct (serde + toml)
├── output/
│   ├── mod.rs           # OutputFormat enum; render() dispatcher
│   └── json.rs          # to_writer(stdout, &result)
└── platform/
    └── macos.rs         # PROTECTED_PATHS constant; cfg-gated
```

The full module tree from ARCHITECTURE.md (`fs_scan/`, `classify/`, `analyze/`, `cleanup/`) is **not built in Phase 1**. Those modules are created in Phases 3-7. Phase 1 creates only the infrastructure skeleton.

### Pattern 1: clap derive — global `--json` flag with nested subcommands

**What:** `Cli` struct holds the global `--json` flag with `global = true`. `Commands` enum contains all top-level subcommands. `Clean` variant wraps a nested `CleanCommands` enum for `preview`/`apply`.

**When to use:** This is the only supported pattern for the described CLI shape.

**Source:** [clap derive tutorial](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html) — `global = true` propagates the flag into all subcommands automatically.

```rust
// src/cli.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "freespace",
    about = "macOS disk inspection and cleanup utility",
    version,
    author
)]
pub struct Cli {
    /// Output results as JSON (stdout only; errors remain on stderr)
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show disk usage summary for all mounted volumes
    Summary,
    /// Scan a path and report size breakdown
    Scan { path: std::path::PathBuf },
    /// Show largest files and directories at a path
    Largest { path: std::path::PathBuf },
    /// Group disk usage into categories at a path
    Categories { path: std::path::PathBuf },
    /// List hidden files and directories at a path
    Hidden { path: std::path::PathBuf },
    /// Discover cache directories across standard macOS locations
    Caches,
    /// Cleanup operations (preview or apply)
    Clean {
        #[command(subcommand)]
        command: CleanCommands,
    },
    /// Show and edit Freespace configuration
    Config,
    /// Run self-diagnostics
    Doctor,
}

#[derive(Subcommand)]
pub enum CleanCommands {
    /// Show files that would be removed (read-only)
    Preview,
    /// Execute cleanup (moves to Trash by default)
    Apply {
        /// Permanently delete instead of moving to Trash
        #[arg(long)]
        force: bool,
    },
}
```

**Access in main.rs:**
```rust
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // cli.json is accessible here AND was visible to any subcommand that used
    // #[arg(from_global)] -- but for this project, main.rs reads it and passes
    // it to command handlers as a plain bool parameter.
    match cli.command {
        Commands::Summary => commands::summary::run(cli.json)?,
        Commands::Clean { command } => match command {
            CleanCommands::Preview => commands::clean::run_preview(cli.json)?,
            CleanCommands::Apply { force } => commands::clean::run_apply(force, cli.json)?,
        },
        // ...
    }
    Ok(())
}
```

### Pattern 2: tracing-subscriber wired to stderr

**What:** Initialize logging in `main.rs` before any other work. All log output (`warn!`, `error!`, `debug!`) goes to stderr, never stdout.

**Why not env_logger:** env_logger writes to stdout by default. Enforcing stderr requires configuration that is easy to forget. tracing-subscriber's `with_writer(io::stderr)` makes stderr the unconditional target.

**Source:** [tracing-subscriber SubscriberBuilder docs](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.SubscriberBuilder.html)

```rust
// src/main.rs — logging init
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() {
    fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("warn"))
        )
        .with_target(false)
        .init();
}
```

`RUST_LOG=debug freespace scan ~/Downloads` routes all debug output to stderr; `freespace scan --json ~/Downloads | jq` is clean.

### Pattern 3: Protected-path constants via `canonicalize()`

**What:** Resolve the five protected paths once at startup using `std::fs::canonicalize` (which calls POSIX `realpath` on macOS). Store the results as `Vec<PathBuf>`. All path comparisons in later phases check against these resolved paths.

**Why `canonicalize()` matters:** `/private/tmp` on macOS is the canonical form of `/tmp` (which is a symlink). Without canonicalization, a user passing `/tmp/something` bypasses a check for `/private`. `realpath` resolves all symlinks in the path, making comparison unambiguous.

**Important caveat:** `std::fs::canonicalize` requires the path to exist. The five macOS protected paths (/System, /usr, /bin, /sbin, /private) always exist on a standard macOS installation. If any fails (e.g., non-standard environment), log a warning and store the raw path as fallback — do not crash.

```rust
// src/platform/macos.rs
#[cfg(target_os = "macos")]
pub mod macos {
    use std::path::PathBuf;

    /// The five protected root paths, canonicalized at startup.
    /// Paths that cannot be canonicalized (non-standard environments)
    /// are stored as-is with a warning.
    pub fn protected_paths() -> Vec<PathBuf> {
        const RAW: &[&str] = &["/System", "/usr", "/bin", "/sbin", "/private"];
        RAW.iter()
            .map(|raw| {
                std::fs::canonicalize(raw).unwrap_or_else(|e| {
                    tracing::warn!("Could not canonicalize protected path {raw}: {e}");
                    PathBuf::from(raw)
                })
            })
            .collect()
    }

    /// Returns true if the given path is inside a protected path.
    pub fn is_protected(path: &std::path::Path, protected: &[PathBuf]) -> bool {
        protected.iter().any(|p| path.starts_with(p))
    }
}
```

**Usage in main.rs:**
```rust
#[cfg(target_os = "macos")]
let protected = platform::macos::protected_paths();
```

### Pattern 4: Config loading with graceful missing-file handling

**What:** Attempt to read `~/.config/Freespace/config.toml`. If missing, return `Config::default()`. If present but malformed, return an error (user must fix it).

**`dirs::config_dir()` on macOS:** Returns `~/Library/Application Support`, not `~/.config`. However, the PRD explicitly mandates `~/.config/Freespace/config.toml`. Use `dirs::home_dir()` and construct the XDG path manually, or use `dirs::config_dir()` if you want the macOS-native location. **PRD wins: use `$HOME/.config/Freespace/config.toml`.**

To get the XDG path regardless of platform behaviour, prefer:
```rust
let config_path = dirs::home_dir()
    .map(|h| h.join(".config/Freespace/config.toml"));
```

**Config schema:**
```rust
// src/config/schema.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub scan: ScanConfig,
    #[serde(default)]
    pub cleanup: CleanupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CleanupConfig {
    #[serde(default)]
    pub safe_categories: Vec<String>,
}
```

**Loader:**
```rust
// src/config/mod.rs
use std::path::Path;
use anyhow::Context;

pub fn load_config() -> anyhow::Result<crate::config::schema::Config> {
    let path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve home directory"))?
        .join(".config/Freespace/config.toml");

    if !path.exists() {
        tracing::debug!("No config file at {}; using defaults", path.display());
        return Ok(Default::default());
    }

    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    toml::from_str(&contents)
        .with_context(|| format!("Config file is malformed: {}", path.display()))
}
```

### Pattern 5: `cfg`-gated platform module

**What:** `src/platform/macos.rs` is gated so its contents compile only on macOS. The `platform` module in `src/platform/mod.rs` re-exports a stable public surface that works on any OS (even if stubbed elsewhere).

```rust
// src/platform/mod.rs
#[cfg(target_os = "macos")]
pub mod macos;

// Re-export types used by the rest of the codebase
#[cfg(target_os = "macos")]
pub use macos::protected_paths;
```

For Phase 1, no Linux/Windows stubs are needed. The `#[cfg]` gate means the code simply doesn't compile on non-macOS, which is acceptable for a macOS-only v1.

### Pattern 6: stdout/stderr separation in `output` module

**What:** The `output` module is the only place in the codebase that writes to stdout. All other code uses `tracing::warn!`, `tracing::error!`, or `eprintln!` (for progress).

```rust
// src/output/mod.rs
use std::io::{self, Write};

pub enum OutputFormat {
    Table,
    Json,
}

impl OutputFormat {
    pub fn from_flag(json: bool) -> Self {
        if json { OutputFormat::Json } else { OutputFormat::Table }
    }
}

pub fn write_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    let stdout = io::stdout();
    serde_json::to_writer(stdout.lock(), value)?;
    // Print a trailing newline so downstream tools see a complete line
    writeln!(io::stdout().lock())?;
    Ok(())
}
```

Command stubs (FOUND-01) call `output::write_json(&serde_json::json!({"status": "not implemented"}))` when `--json` is set, and `eprintln!("not implemented")` otherwise. This validates the wiring before real logic exists.

### Anti-Patterns to Avoid

- **`println!` in command handlers:** Routes to stdout, polluting JSON pipelines. Use the `output` module exclusively.
- **`env_logger` for logging:** Writes to stdout by default; awkward to redirect. Use tracing-subscriber with explicit `with_writer(io::stderr)`.
- **Calling `canonicalize()` per-request:** Expensive syscall. Call once at startup, store results in a `Vec<PathBuf>` passed through `Config` or held in `main`.
- **`fs::canonicalize` on a non-existent path:** Returns an error. The five macOS protected paths always exist on a real macOS system, but tests run on tmpdir — use a fallback as shown in Pattern 3.
- **Hardcoding `~/Library/Application Support` for config:** The PRD mandates `~/.config/Freespace/config.toml`. Follow the PRD.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI argument parsing | Custom arg parser | clap 4.6 derive | clap handles `--help`, `--version`, error messages, shell completions, type coercion |
| Config deserialization | Manual TOML parser | `toml` + serde derive | Handles nested structs, optional fields, defaults, error messages |
| Platform path resolution | Hardcoded `$HOME` + string concat | `dirs::home_dir()` | Handles edge cases (no HOME set, sandboxed environments) |
| Logging to stderr | Manual `eprintln!` formatting | tracing + tracing-subscriber | Structured, filterable, respects RUST_LOG, easily tested |
| Error types | `Box<dyn Error>` or string errors | thiserror + anyhow | Type-safe matching in library code; ergonomic propagation in handlers |

**Key insight:** The infrastructure for a Rust CLI is almost entirely provided by a small set of battle-tested crates. The value added in Phase 1 is the wiring and configuration, not novel code.

---

## Common Pitfalls

### Pitfall 1: `--json` not reaching nested subcommands

**What goes wrong:** User runs `freespace clean preview --json` and the flag is silently ignored because it appears after the nested subcommand.

**Why it happens:** Without `global = true`, clap scopes flags to the struct/variant they are declared in. A flag on `Cli` does not automatically propagate into `CleanCommands`.

**How to avoid:** Declare `#[arg(long, global = true)]` on the `json` field in `Cli`. With this attribute, clap propagates the flag through the entire subcommand tree regardless of where it appears on the command line.

**Warning signs:** `--json` flag not listed in `freespace clean preview --help`.

### Pitfall 2: `canonicalize()` returning an error in tests

**What goes wrong:** Tests that call `is_protected()` or `protected_paths()` fail because the paths `/System`, `/usr`, etc. don't exist in the test sandbox (CI, Docker containers, etc.).

**Why it happens:** `std::fs::canonicalize` returns `Err` if the path doesn't exist.

**How to avoid:** Wrap `canonicalize` with `unwrap_or_else(|_| PathBuf::from(raw))` as shown in Pattern 3. Tests that need deterministic protected paths should construct `Vec<PathBuf>` directly from literals rather than calling `protected_paths()`.

### Pitfall 3: `dirs::config_dir()` returning the macOS app support path

**What goes wrong:** Config file is written to `~/Library/Application Support/Freespace/config.toml` instead of `~/.config/Freespace/config.toml`.

**Why it happens:** `dirs::config_dir()` follows macOS Standard Directory guidelines, returning `~/Library/Application Support` on macOS — not the XDG path.

**How to avoid:** Use `dirs::home_dir().map(|h| h.join(".config/Freespace/config.toml"))` to construct the XDG path explicitly, as mandated by the PRD.

### Pitfall 4: Config parse error crashes the tool

**What goes wrong:** User has a malformed `config.toml`; tool exits with an opaque error rather than a helpful message.

**Why it happens:** `toml::from_str` returns an error with the TOML parse location. If propagated through `?` in main without context, the error message is bare.

**How to avoid:** Wrap the `toml::from_str` call with `.with_context(|| format!("Config file is malformed: {}", path.display()))` using anyhow. Emit the error to stderr and use the default config as a fallback if desired.

### Pitfall 5: Logging init called after first log event

**What goes wrong:** A `warn!` call happens during config loading, before `init_logging()` is called in main. The event is silently dropped.

**Why it happens:** tracing events before subscriber initialization are dropped (not buffered).

**How to avoid:** `init_logging()` must be the very first call in `main()`, before `Cli::parse()`, config loading, or any other setup.

---

## Code Examples

### Verified patterns

#### Complete `main.rs` skeleton

```rust
// src/main.rs
// Source: clap derive docs + tracing-subscriber docs (verified 2026-03-28)

mod cli;
mod commands;
mod config;
mod output;
mod platform;

use anyhow::Context;
use clap::Parser;
use cli::{Cli, CleanCommands, Commands};

fn main() -> anyhow::Result<()> {
    // 1. Logging MUST be initialized first — events before init are dropped
    init_logging();

    // 2. Parse CLI (clap prints --help and exits if needed)
    let cli = Cli::parse();

    // 3. Load config — missing file is OK; malformed file is an error
    let config = config::load_config()
        .context("Failed to load configuration")?;

    // 4. Resolve protected paths once (expensive canonicalize syscall)
    #[cfg(target_os = "macos")]
    let _protected = platform::macos::protected_paths();

    // 5. Dispatch
    match cli.command {
        Commands::Summary => commands::summary::run(&config, cli.json),
        Commands::Scan { path } => commands::scan::run(&path, &config, cli.json),
        Commands::Largest { path } => commands::largest::run(&path, &config, cli.json),
        Commands::Categories { path } => commands::categories::run(&path, &config, cli.json),
        Commands::Hidden { path } => commands::hidden::run(&path, &config, cli.json),
        Commands::Caches => commands::caches::run(&config, cli.json),
        Commands::Clean { command } => match command {
            CleanCommands::Preview => commands::clean::run_preview(&config, cli.json),
            CleanCommands::Apply { force } => commands::clean::run_apply(force, &config, cli.json),
        },
        Commands::Config => commands::config::run(&config, cli.json),
        Commands::Doctor => commands::doctor::run(&config, cli.json),
    }
}

fn init_logging() {
    use tracing_subscriber::{fmt, EnvFilter};
    fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("warn"))
        )
        .with_target(false)
        .init();
}
```

#### Stub command handler pattern

```rust
// src/commands/summary.rs (stub — real logic added in Phase 2)
use crate::config::schema::Config;

pub fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    if json {
        crate::output::write_json(&serde_json::json!({
            "status": "not_implemented",
            "command": "summary"
        }))?;
    } else {
        eprintln!("summary: not yet implemented");
    }
    Ok(())
}
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `structopt` for CLI parsing | clap 4.6 derive (`#[derive(Parser)]`) | structopt merged into clap v3 in 2021; clap v4 is the current standard |
| `env_logger` for logging | tracing + tracing-subscriber | env_logger writes to stdout; tracing-subscriber writes to stderr explicitly |
| `colored` crate for colors | `owo-colors` 4.x | colored abandoned 2022; owo-colors actively maintained |
| Manual TOML parsing | `toml` 1.1 + serde derive | toml 1.x implements TOML 1.1 spec; earlier versions had spec gaps |
| `log` facade alone | `tracing` 0.1 | `log` requires a separate runtime; tracing provides structured events |

**Deprecated/outdated:**
- `structopt`: Do not use. It re-exports clap internally and blocks clap v4 features.
- `env_logger`: Do not use. It logs to stdout by default; enforcing stderr is fragile.
- `glob` crate: Unmaintained since 2019. Use `walkdir` filter closures.

---

## Open Questions

1. **`clean` subcommand naming in `--help`**
   - What we know: clap uses the variant name (lowercase) by default → `clean` with subcommands `preview` and `apply`
   - What's unclear: Whether the user expects `freespace clean-preview` (hyphenated) vs `freespace clean preview` (space-separated nested subcommand)
   - Recommendation: Use the nested `Clean { command: CleanCommands }` pattern as researched. This produces `freespace clean preview` and `freespace clean apply`, matching the PRD language.

2. **`config` subcommand conflicts with `config` module**
   - What we know: Rust allows a function named `run` in `commands::config` while the module `config` also exists at the crate root
   - What's unclear: Whether the namespace collision between `Commands::Config` and `mod config` causes confusion
   - Recommendation: Name the command handler module `commands/config_cmd.rs` or ensure imports are explicit. This is a cosmetic concern, not a correctness issue.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) + `cargo-nextest` |
| Config file | None needed — Cargo.toml `[dev-dependencies]` |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo nextest run` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FOUND-01 | `freespace --help` lists all subcommands | smoke | `cargo test cli::tests::all_subcommands_in_help` | Wave 0 |
| FOUND-01 | `freespace clean --help` lists `preview` and `apply` | smoke | `cargo test cli::tests::clean_subcommands_in_help` | Wave 0 |
| FOUND-02 | `platform::macos` module compiles only on macOS | unit (cfg) | `cargo test` (cfg gates enforce this at compile time) | Wave 0 |
| FOUND-03 | `/tmp` resolves to `/private/tmp` via `canonicalize` | unit | `cargo test platform::macos::tests::tmp_canonicalizes` | Wave 0 |
| FOUND-03 | `is_protected("/System/something")` returns true | unit | `cargo test platform::macos::tests::is_protected_system` | Wave 0 |
| FOUND-04 | Missing config file returns `Config::default()` | unit | `cargo test config::tests::missing_file_returns_default` | Wave 0 |
| FOUND-04 | Malformed config returns an error (not panic) | unit | `cargo test config::tests::malformed_file_returns_error` | Wave 0 |
| FOUND-05 | No output to stdout from logging macros | unit | `cargo test output::tests::no_log_on_stdout` | Wave 0 |
| FOUND-06 | `--json` flag present in `freespace --help` | smoke | `cargo test cli::tests::json_flag_in_help` | Wave 0 |
| FOUND-06 | `--json` flag present in `freespace clean preview --help` | smoke | `cargo test cli::tests::json_flag_propagates_to_nested` | Wave 0 |

**Note on smoke tests for clap:** clap 4.x provides `assert_cmd` and `.try_parse_from()` for in-process CLI testing without spawning a subprocess. Use `Cli::try_parse_from(["freespace", "--help"])` in unit tests to validate help output structure.

### Sampling Rate

- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo nextest run`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `src/cli.rs` — must be created with `#[cfg(test)] mod tests` block covering FOUND-01, FOUND-06
- [ ] `src/config/mod.rs` — must include `#[cfg(test)] mod tests` covering FOUND-04
- [ ] `src/platform/macos.rs` — must include `#[cfg(test)] mod tests` covering FOUND-03
- [ ] Dev dependency: `assert_cmd = "2.0"` in `[dev-dependencies]` for CLI smoke tests
- [ ] Dev dependency: `tempfile = "3.27"` for config tests using temp TOML files

---

## Sources

### Primary (HIGH confidence)
- [clap derive tutorial — docs.rs](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html) — derive syntax, `global = true` flag pattern, nested subcommands
- [tracing-subscriber SubscriberBuilder — docs.rs](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/struct.SubscriberBuilder.html) — `with_writer(io::stderr)` pattern
- [std::fs::canonicalize — Rust std](https://doc.rust-lang.org/std/fs/fn.canonicalize.html) — POSIX `realpath` behavior, error conditions
- crates.io API — tracing 0.1.44 and tracing-subscriber 0.3.23 verified 2026-03-28
- `.planning/research/STACK.md` — all dependency versions verified 2026-03-28

### Secondary (MEDIUM confidence)
- [dirs crate — docs.rs](https://docs.rs/dirs/latest/dirs/) — `config_dir()` macOS behavior (returns `~/Library/Application Support`)
- [Rain's Rust CLI Recommendations](https://rust-cli-recommendations.sunshowers.io/handling-arguments.html) — argument handling patterns

### Tertiary (LOW confidence)
- None

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — versions verified against crates.io API directly
- Architecture: HIGH — drawn from ARCHITECTURE.md (2026-03-28) and official clap docs
- Pitfalls: HIGH — most verified against official docs (canonicalize error conditions, dirs behavior on macOS)
- Test strategy: MEDIUM — test file names and command strings are prescriptive but untested until Wave 0

**Research date:** 2026-03-28
**Valid until:** 2026-04-28 (stable crates; clap, serde, tracing change infrequently)
