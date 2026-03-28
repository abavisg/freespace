# Stack Research

**Domain:** Rust CLI disk inspection and cleanup utility (macOS)
**Researched:** 2026-03-28
**Confidence:** HIGH (all versions verified against crates.io API)

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Rust | stable (1.88+) | Implementation language | Memory safety without GC, zero-cost abstractions for large directory traversal, no runtime overhead. `sysinfo` requires rustc 1.88 minimum as of 0.38.x. |
| clap | 4.6.0 | CLI argument parsing, subcommands, help generation | Dominant Rust CLI crate. v4 derive macro API eliminates boilerplate: annotate structs with `#[derive(Parser, Subcommand)]` and clap handles parsing, validation, `--help`, and shell completion generation. Builder pattern still available for edge cases. |
| walkdir | 2.5.0 | Recursive directory traversal | BurntSushi's crate is the de-facto standard: deterministic ordering, controlled symlink following, configurable max depth, and proper handling of permission errors without panicking. Matches `find` performance on local FS. Used by ripgrep internally. |
| serde | 1.0.228 | Serialization/deserialization framework | Universal Rust serialization. Required by both `serde_json` and `toml`. Zero-cost at runtime through derive macros. |
| serde_json | 1.0.149 | JSON output serialization | Pair with serde derives. All `--json` output written to stdout via `serde_json::to_writer(stdout, ...)`. Errors and logs always go to stderr — this separation is enforced by the output contract. |
| toml | 1.1.0 | Config file parsing (`~/.config/Freespace/config.toml`) | Official TOML 1.1 spec implementation. Deserializes directly into serde-annotated structs. No separate parser step needed. |
| trash | 5.2.5 | Move files to macOS Trash (safe deletion) | Maintained by Byron (active as of 2025-02). Integrates with macOS Finder Trash natively — files moved via `trash::delete` appear in Trash and are recoverable. Only correct implementation of macOS Trash in Rust; no alternatives exist at this quality. |
| thiserror | 2.0.18 | Typed error enums for domain errors | Use thiserror for all domain-layer error types (`ScanError`, `CleanupError`, etc.) where callers need to match on error variants. v2.0 is the current major with improved derive ergonomics. |
| anyhow | 1.0.102 | Error propagation in binary entry points | Use anyhow in `main.rs` and top-level command handlers where errors are displayed to the user rather than programmatically matched. Pairs with thiserror: library modules define typed errors, main aggregates with anyhow. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| comfy-table | 7.2.2 | Terminal table rendering with alignment and borders | All tabular human-readable output: volume list, scan results, category breakdown. Respects terminal width, handles Unicode. Do not use for `--json` output paths. |
| indicatif | 0.18.4 | Progress bars and spinners | Show scan progress on large directories (100k+ files). Write to stderr so it does not pollute `--json` stdout. Use `ProgressBar::with_message` for file-count updates. |
| owo-colors | 4.3.0 | Terminal color output | Lightweight, no_std-compatible color library. Preferred over `colored` (abandoned) and `termcolor` (verbose). Check `NO_COLOR` env var support is built-in from v3+. |
| sysinfo | 0.38.4 | Mounted volume enumeration (total/used/available space) | The `Disks` API provides mount point, total space, and available space for all mounted volumes on macOS. Note: `statvfs`-based I/O is blocking — call on a background thread if doing live refresh. |
| dirs | 6.0.0 | XDG/macOS standard path resolution | Resolve `~/.config/Freespace/config.toml` and `~/.local/state/Freespace/cleanup.log` via `dirs::config_dir()` and `dirs::state_dir()` rather than hardcoding `$HOME`. |
| rayon | 1.11.0 | Parallel iteration | Parallelize directory scanning across top-level subdirectories when traversing very large trees. Do not use for the per-file walk itself (walkdir is sequential by design); use rayon to fan out across multiple root paths. |
| bytesize | 2.3.1 | Human-readable byte formatting (KB/MB/GB) | Format file sizes consistently across table and JSON output. `ByteSize::b(n).to_string()` gives `"1.5 GiB"` style output. |
| tempfile | 3.27.0 | Temporary files/dirs in tests | Integration test isolation — create temp directory trees for scan tests without touching real FS. |
| nix | 0.31.2 | Low-level macOS syscalls (statvfs, lstat) | Use only for platform::macos module when sysinfo does not expose needed detail (e.g., filesystem type strings, inode counts). Keep nix behind `#[cfg(target_os = "macos")]`. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| cargo clippy | Linting and idiomatic Rust enforcement | Run as `cargo clippy -- -D warnings` in CI. Catches performance and correctness issues specific to iterator chains and error handling. |
| cargo fmt | Formatting | Enforce `rustfmt.toml` at repo root. No style debates. |
| cargo nextest | Faster test runner | Drop-in replacement for `cargo test`. Parallelizes test execution; critical for integration tests that exercise FS operations. Install: `cargo install cargo-nextest`. |
| cargo-deny | Dependency audit (licenses, advisories, duplicates) | Run in CI to block crates with security advisories or incompatible licenses. Config at `deny.toml`. |
| cargo-dist | Release binary packaging | Produces GitHub Releases with prebuilt macOS binaries (universal, aarch64, x86_64). Configured via `[workspace.metadata.dist]` in `Cargo.toml`. |

## Installation

```toml
# Cargo.toml [dependencies]
clap = { version = "4.6", features = ["derive"] }
walkdir = "2.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "1.1"
trash = "5.2"
thiserror = "2.0"
anyhow = "1.0"
comfy-table = "7.2"
indicatif = "0.18"
owo-colors = "4.3"
sysinfo = "0.38"
dirs = "6.0"
rayon = "1.11"
bytesize = "2.3"
nix = { version = "0.31", features = ["fs"], optional = true }

[dev-dependencies]
tempfile = "3.27"
```

```bash
# Install dev tools
cargo install cargo-nextest cargo-deny cargo-dist
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| walkdir 2.5 | ignore 0.4.25 | Use `ignore` only if you need parallel walk with `.gitignore` respect baked in (e.g., a code-search tool). For a disk utility that must scan *everything* including hidden files and ignored dirs, `ignore`'s filtering is a liability. walkdir's sequential model also makes streaming aggregation simpler. |
| walkdir 2.5 | jwalk 0.8.1 | `jwalk` is parallel-first and faster on NVMe. Consider for a future optimization phase if single-threaded scan becomes a bottleneck. API is less stable than walkdir and parallel ordering is non-deterministic. |
| thiserror 2.0 | snafu | `snafu` offers more context-threading features but is significantly more verbose. thiserror covers 95% of cases with less ceremony. |
| owo-colors 4.3 | colored | `colored` is effectively abandoned (last release 2022). `owo-colors` is actively maintained and is no_std-compatible. |
| sysinfo 0.38 | libc statvfs directly | Direct libc calls require unsafe and platform-specific handling. sysinfo wraps this correctly and is cross-platform if macOS-only ever expands. |
| comfy-table 7.2 | tabled | Both are viable. comfy-table has better terminal-width awareness and is more battle-tested in CLI tools. tabled has a richer styling API but is overkill here. |
| toml 1.1 | config (config-rs) | `config` is appropriate when layering multiple config sources (env vars, files, CLI flags). For a single `config.toml` with known structure, `toml` + serde is simpler and has no runtime overhead. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| structopt | Merged into clap v3 in 2021. Still compiles but re-exports clap. Using it adds confusion and blocks clap v4 features. | clap 4.x with `#[derive(Parser)]` |
| colored | Last crates.io release was 2022, known terminal detection bugs on macOS. | owo-colors 4.x |
| env_logger | Logs to stdout. For a CLI with `--json` output, logs must go to stderr only. env_logger makes this awkward to enforce. | tracing + tracing-subscriber (configure stderr writer explicitly) |
| log (facade only) | The `log` crate facade without a subscriber does nothing at runtime. Use tracing ecosystem for structured output. | tracing 0.1 + tracing-subscriber 0.3 |
| std::fs::remove_file | Permanent deletion with no Trash integration and no recovery. Violates the trash-first safety model. | `trash::delete` for safe deletion; `std::fs::remove_file` only behind explicit `--force` guard after protected-path check |
| glob | Pattern matching for file paths via shell globs. Unmaintained since 2019. | Manual path matching with `std::path::Path` or `walkdir` filter closures |
| sys-info | Old crate (`sys_info`), C FFI-heavy, infrequent updates. Disk info API is minimal. | sysinfo 0.38 |

## Stack Patterns by Variant

**If scanning a single large directory (100k+ files):**
- Use walkdir with streaming aggregation — do not collect all entries into a Vec before processing
- Increment counters in-place: `total_size += entry.metadata()?.len()`
- Emit indicatif progress updates every N entries (e.g., every 1000) to avoid syscall overhead

**If implementing `--json` output:**
- Construct the full result struct in memory, then call `serde_json::to_writer(std::io::stdout(), &result)?`
- All `eprintln!`, indicatif progress bars, and tracing output must be bound to stderr
- Never mix human-readable formatting into the JSON code path

**If adding macOS-specific volume info:**
- Gate nix/libc calls behind `#[cfg(target_os = "macos")]` inside `src/platform/macos.rs`
- The `platform` module exposes a trait; the rest of the codebase calls the trait, not the platform impl directly

**If running tests on FS operations:**
- Use `tempfile::tempdir()` to create isolated test environments
- Never test against real user directories — tests must be hermetic

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| serde 1.0 | serde_json 1.0, toml 1.1 | All use serde 1.x trait bounds. No conflicts. |
| clap 4.6 | No conflicts with other listed crates | clap 4.x is a complete rewrite from 3.x; do not mix clap 3 and 4 in the same binary. |
| thiserror 2.0 | anyhow 1.0 | thiserror errors implement `std::error::Error`; anyhow wraps anything that does. Fully compatible. |
| sysinfo 0.38 | Requires rustc 1.88+ | Minimum Rust version bump in 0.38.x. Use `rust-toolchain.toml` to pin stable channel at ≥1.88. |
| nix 0.31 | libc 0.2 (transitive) | nix brings in libc as a dep. No conflict with other crates listed. |
| trash 5.2 | macOS 10.13+ | trash-rs uses AppleScript/Scripting Bridge on macOS. No known compatibility issues with modern macOS versions. |

## Sources

- crates.io API (`/api/v1/crates/{name}`) — all version numbers verified directly, 2026-03-28
- [Rust CLI Patterns (2026-02)](https://dasroot.net/posts/2026/02/rust-cli-patterns-clap-cargo-configuration/) — clap derive pattern confirmation
- [walkdir GitHub (BurntSushi)](https://github.com/BurntSushi/walkdir) — sequential walk design rationale, MEDIUM confidence
- [trash-rs GitHub (Byron)](https://github.com/Byron/trash-rs) — macOS Trash integration, MEDIUM confidence
- [Rust Error Handling 2025 Guide](https://markaicode.com/rust-error-handling-2025-guide/) — thiserror 2.0 / anyhow pattern for CLIs, MEDIUM confidence
- [sysinfo Disks API docs](https://docs.rs/sysinfo/latest/sysinfo/) — Disks struct, statvfs blocking note, HIGH confidence
- [thiserror vs anyhow for CLI apps](https://www.shakacode.com/blog/thiserror-anyhow-or-how-i-handle-errors-in-rust-apps/) — layering pattern, MEDIUM confidence

---
*Stack research for: Freespace — Rust CLI disk inspection and cleanup tool (macOS)*
*Researched: 2026-03-28*
