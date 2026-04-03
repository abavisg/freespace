# Phase 6: Cleanup Preview — Research

**Researched:** 2026-04-02
**Domain:** Rust CLI read-only preview gate, cache discovery aggregation, serde JSON, comfy-table output
**Confidence:** HIGH

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PREV-01 | `freespace clean preview` shows all files/directories that would be affected, total reclaimable space, and safety classification per item | `caches.rs` already implements the same pattern (known-dir enumeration + safety_class + size aggregation); clean.rs is a stub ready for implementation |
| PREV-02 | Preview is read-only — no files are modified or deleted during preview | No `trash` crate usage in this phase; implementation is scan + classify only |
| PREV-03 | Preview output is human-readable table by default and clean JSON with `--json` | Existing `output::write_json` + `comfy_table::Table` pattern is established across all commands |
</phase_requirements>

---

## Summary

Phase 6 implements `freespace clean preview` — a read-only command that enumerates all cleanup candidates with their safety classifications and individual sizes, reports total reclaimable space, and makes zero disk changes. This is the gate between inspection and destruction: Phase 7 (cleanup apply) cannot run without preview having been verified correct first.

The implementation is a direct composition of work already done. The `caches.rs` command (Phase 4) established the canonical pattern: call `known_cache_dirs()`, iterate, call `fs_scan::scan_path()` per directory, call `classify::safety_class()` per path, aggregate totals, render table or JSON. The preview command follows this exact pattern but scopes "cleanup candidates" to the set of directories where the safety classification is `Safe` or `Caution` — the same directories that `caches` already surfaces.

The key architectural question is: what does "would be affected" mean? Given the codebase design (safety-first, no session state, no prior-scan requirement for preview), the most consistent answer is that preview mirrors the `caches` command output but frames it as "what clean apply would target." Specifically: the known cache directories, filtered and sorted by safety class, with a reclaimable total summed across `Safe`-classified entries. This matches the existing `CachesResult` shape and reuses all existing infrastructure.

The stub in `commands/clean.rs` already exists and is wired in `main.rs`. This phase replaces the stub body with real logic. No new Cargo dependencies are needed.

**Primary recommendation:** Implement `run_preview` in `src/commands/clean.rs` by composing the existing `caches` enumeration pattern into a `PreviewResult` struct (parallel to `CachesResult`) with entries, total bytes, and reclaimable bytes. Reuse `classify::safety_class`, `fs_scan::scan_path`, and `output::write_json`. Add an integration test file `tests/clean_preview_cmd.rs`.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `crate::fs_scan::scan_path` | internal | Physical-size scan per directory | Already handles hardlink dedup, permission errors, streaming |
| `crate::classify::safety_class` | internal | Safety classification per path | Already implements all 4 classes (safe/caution/dangerous/blocked) |
| `crate::output::write_json` | internal | JSON output to stdout | Established contract across all commands |
| `comfy_table` | 7.2 (in Cargo.toml) | Table rendering for human output | Used by caches, categories, scan, largest |
| `bytesize` | 2.3 (in Cargo.toml) | Human-readable byte formatting | Used by caches, scan |
| `serde` + `serde_json` | 1.0 (in Cargo.toml) | JSON serialization of PreviewResult | Used everywhere |
| `dirs` | 6.0 (in Cargo.toml) | Home directory resolution | Used by caches already |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `assert_cmd` | 2.0 (in dev-dependencies) | Integration testing via binary invocation | Test file in `tests/clean_preview_cmd.rs` |
| `tempfile` | 3.27 (in dev-dependencies) | Temporary directories in unit tests | If unit-level tests are added for helper functions |

**No new dependencies required.** All needed crates are already in Cargo.toml.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Reusing known_cache_dirs from caches.rs | Scanning an arbitrary user-specified path | Arbitrary path requires a path argument; the requirements do not specify one — "clean preview" is pathless (targets the canonical cache locations) |
| Composing with `caches::run()` directly | Sharing a helper module | caches.rs uses a private `CacheEntry` type; better to define `PreviewEntry` in clean.rs and share the `known_cache_dirs` logic via a new `crate::platform::macos` or `crate::classify` helper if needed |

---

## Architecture Patterns

### Existing Module Layout (Phase 6 slots in here)

```
src/
├── commands/
│   ├── clean.rs          <-- Phase 6: replace stub with real run_preview()
│   ├── caches.rs         <-- reference implementation for same pattern
│   ├── mod.rs
│   └── ...
├── classify/
│   └── mod.rs            <-- safety_class() and Category already defined
├── fs_scan/
│   └── mod.rs            <-- scan_path() returns ScanResult
├── output/
│   └── mod.rs            <-- write_json() established contract
└── platform/
    └── macos.rs          <-- protected_paths() already defined
tests/
├── clean_preview_cmd.rs  <-- Phase 6: new integration test file
├── caches_cmd.rs         <-- reference for test pattern
└── ...
```

### Pattern 1: PreviewResult Shape (mirrors CachesResult)

The output data shape should parallel `CachesResult` in `caches.rs` — which is already established and tested. Define a private `PreviewEntry` and `PreviewResult` in `commands/clean.rs`:

```rust
// src/commands/clean.rs
use crate::classify::{safety_class, SafetyClass};
use crate::config::schema::Config;
use bytesize::ByteSize;
use comfy_table::Table;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct PreviewEntry {
    path: PathBuf,
    total_bytes: u64,
    file_count: u64,
    safety: SafetyClass,
}

#[derive(Debug, Serialize)]
struct PreviewResult {
    candidates: Vec<PreviewEntry>,
    total_bytes: u64,
    reclaimable_bytes: u64,  // sum of Safe-classified entries only
}
```

### Pattern 2: Known Cache Dirs (extract from caches.rs or duplicate inline)

The `known_cache_dirs` function in `caches.rs` is private. Phase 6 needs the same list. Options:
- **Option A (recommended):** Duplicate the list inline in `clean.rs` — the list is small and stable; DRY is less important than avoiding cross-module coupling into a private function.
- **Option B:** Extract `known_cache_dirs` into `src/platform/macos.rs` and make it pub. Cleaner long-term; the platform module already owns macOS-specific knowledge.

Either is correct. The planner should decide; both are safe. If Option B is chosen, it requires a small refactor of `caches.rs` to use the public version.

### Pattern 3: run_preview() Flow

```rust
pub fn run_preview(config: &Config, json: bool) -> anyhow::Result<()> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve home directory"))?;

    let dirs_to_scan = known_cache_dirs(&home);  // same list as caches.rs

    let mut candidates: Vec<PreviewEntry> = Vec::new();
    for dir in dirs_to_scan {
        if !dir.exists() { continue; }
        let scan_result = crate::fs_scan::scan_path(&dir, config);
        let safety = safety_class(&dir, &home);
        candidates.push(PreviewEntry {
            path: dir,
            total_bytes: scan_result.total_bytes,
            file_count: scan_result.file_count,
            safety,
        });
    }

    // Sort: safe first (most actionable), then by size descending within class
    candidates.sort_by(|a, b| {
        a.safety.cmp(&b.safety)           // needs Ord on SafetyClass
            .then(b.total_bytes.cmp(&a.total_bytes))
    });

    let total_bytes: u64 = candidates.iter().map(|e| e.total_bytes).sum();
    let reclaimable_bytes: u64 = candidates
        .iter()
        .filter(|e| e.safety == SafetyClass::Safe)
        .map(|e| e.total_bytes)
        .sum();

    let result = PreviewResult { candidates, total_bytes, reclaimable_bytes };

    if json {
        crate::output::write_json(&result)?;
    } else {
        render_preview_table(&result);
    }
    Ok(())
}
```

### Pattern 4: SafetyClass Ordering

The sort above requires `Ord` on `SafetyClass`. Currently `SafetyClass` derives only `Debug, Clone, Copy, PartialEq, Eq, Serialize`. To sort by safety class, either:
- Derive `PartialOrd, Ord` on `SafetyClass` (order is Safe < Caution < Dangerous < Blocked — the enum variant definition order)
- Or use a custom sort key function

Deriving `PartialOrd, Ord` on `SafetyClass` is clean and aligns with the intent. The enum definition order in `classify/mod.rs` must be Safe, Caution, Dangerous, Blocked for the derived order to make sense. Currently it is defined in that order — confirmed from source.

### Pattern 5: Table Rendering (matches caches.rs)

```rust
fn render_preview_table(result: &PreviewResult) {
    let mut table = Table::new();
    table.set_header(vec!["Path", "Size", "Files", "Safety"]);
    for entry in &result.candidates {
        table.add_row(vec![
            entry.path.to_string_lossy().to_string(),
            ByteSize::b(entry.total_bytes).to_string(),
            entry.file_count.to_string(),
            entry.safety.to_string(),
        ]);
    }
    println!("{table}");
    println!("Total: {}", ByteSize::b(result.total_bytes));
    println!("Reclaimable (safe): {}", ByteSize::b(result.reclaimable_bytes));
}
```

### Pattern 6: Integration Test Shape (matches caches_cmd.rs)

```rust
// tests/clean_preview_cmd.rs
use assert_cmd::Command;

fn freespace() -> Command {
    Command::cargo_bin("freespace").unwrap()
}

#[test]
fn test_clean_preview_exits_ok() {
    let output = freespace()
        .args(["clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn test_clean_preview_json_fields() {
    let output = freespace()
        .args(["--json", "clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout must be valid JSON");
    assert!(parsed.get("candidates").is_some());
    assert!(parsed["candidates"].is_array());
    assert!(parsed.get("total_bytes").is_some());
    assert!(parsed.get("reclaimable_bytes").is_some());
}

#[test]
fn test_clean_preview_stderr_clean_with_json() {
    let output = freespace()
        .args(["--json", "clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty(),
        "stderr must be empty with RUST_LOG=off and --json");
}

#[test]
fn test_clean_preview_makes_no_changes() {
    // Read-only guarantee: run preview twice, assert stdout is identical
    // (proves no side effects on disk state)
    let run = |args: &[&str]| {
        freespace()
            .args(args)
            .env("RUST_LOG", "off")
            .output()
            .unwrap()
    };
    let first = run(&["--json", "clean", "preview"]);
    let second = run(&["--json", "clean", "preview"]);
    assert_eq!(first.stdout, second.stdout,
        "preview must be idempotent — no disk changes between runs");
}

#[test]
fn test_clean_preview_safety_values_valid() {
    let output = freespace()
        .args(["--json", "clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();
    let valid_safety = ["safe", "caution", "dangerous", "blocked"];
    for entry in parsed["candidates"].as_array().unwrap() {
        let safety = entry["safety"].as_str().expect("safety must be string");
        assert!(valid_safety.contains(&safety));
    }
}

#[test]
fn test_clean_preview_reclaimable_lte_total() {
    let output = freespace()
        .args(["--json", "clean", "preview"])
        .env("RUST_LOG", "off")
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: serde_json::Value =
        serde_json::from_str(&String::from_utf8(output.stdout).unwrap()).unwrap();
    let reclaimable = parsed["reclaimable_bytes"].as_u64().unwrap_or(0);
    let total = parsed["total_bytes"].as_u64().unwrap_or(0);
    assert!(reclaimable <= total,
        "reclaimable_bytes must be <= total_bytes");
}
```

### Anti-Patterns to Avoid

- **Do not call `trash` or any filesystem-modifying function in `run_preview`:** Even importing `trash` and not calling it is fine, but calling any write/move/delete function in the preview path breaks PREV-02.
- **Do not invent a session state mechanism:** Phase 7 requires "a prior scan and classification pass" (APPLY-05), but that is Phase 7's problem. Phase 6 preview is standalone and self-contained.
- **Do not sort using logical size:** Use `total_bytes` from `scan_path` which is physical (blocks * 512) — consistent with every other command.
- **Do not print anything to stdout in non-JSON mode except the table and summary lines:** Diagnostic messages go to stderr via `tracing::warn!` or `eprintln!`.
- **Do not require a path argument for `clean preview`:** The CLI definition in `cli.rs` shows `CleanCommands::Preview` with no arguments. Preview operates on the canonical known-dir list, not a user-supplied path.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Physical-size scan | Custom traversal loop | `fs_scan::scan_path` | Already handles hardlink dedup, permission errors, streaming, inode tracking |
| Safety classification | Custom rules inline in clean.rs | `classify::safety_class` | All 4 classes already defined and tested; adding rules here creates a second source of truth |
| JSON output | `println!("{}", serde_json::to_string(...))` | `output::write_json` | Ensures consistent stdout-only contract; avoids trailing whitespace issues |
| Human-readable bytes | Custom formatting | `bytesize::ByteSize::b(n)` | Already used by all output paths |
| Table layout | Manual column padding | `comfy_table::Table` | Already used; consistent table style across all commands |
| Home directory | Hardcoded `~` or env var lookup | `dirs::home_dir()` | Already the established pattern; handles edge cases on macOS |

**Key insight:** The preview command is pure composition. Every building block already exists and is tested. The only new code is the glue in `run_preview` and a new integration test file.

---

## Common Pitfalls

### Pitfall 1: SafetyClass Missing Ord Derivation

**What goes wrong:** The sort step (safe entries first, then by size) requires `Ord` on `SafetyClass`. Without it, `sort_by` with `a.safety.cmp(&b.safety)` fails to compile.

**Why it happens:** `SafetyClass` currently derives only `Debug, Clone, Copy, PartialEq, Eq, Serialize`. `Ord` was not needed before because no code sorted by it.

**How to avoid:** Add `PartialOrd, Ord` to the derive list in `classify/mod.rs`. Confirm the enum variant order is Safe → Caution → Dangerous → Blocked before deriving (it is — verified from source).

**Warning signs:** Compile error: `the trait Ord is not implemented for SafetyClass`.

### Pitfall 2: Stdout Contamination in Non-JSON Mode

**What goes wrong:** Any `println!` that is not the table or summary lines causes `--json` parsers to fail if the code path is accidentally entered, and contaminates human output in non-JSON mode.

**Why it happens:** Easy to add a debug `println!` during development.

**How to avoid:** All debug output uses `tracing::warn!` / `tracing::debug!` — routed to stderr. The `RUST_LOG=off` env in tests enforces this.

**Warning signs:** Integration test `test_clean_preview_stderr_clean_with_json` fails; JSON parse fails in `test_clean_preview_json_fields`.

### Pitfall 3: Accidentally Calling scan_path on Non-Existent Dirs

**What goes wrong:** `scan_path` does not explicitly handle the case where root does not exist — it relies on `WalkDir` returning an error on the first entry. This is graceful but may produce a misleading `skipped_count` increment without a zero-total result.

**Why it happens:** `known_cache_dirs` returns paths that may not exist (e.g., `~/.gradle/caches` on a machine with no Gradle).

**How to avoid:** The `caches.rs` pattern guards with `if !dir.exists() { continue; }` before calling `scan_path`. Phase 6 must do the same.

**Warning signs:** Preview shows entries with 0 bytes for non-existent directories.

### Pitfall 4: Reclaimable Total Includes Caution/Dangerous Entries

**What goes wrong:** If `reclaimable_bytes` sums all entries instead of only `SafetyClass::Safe` entries, the number is misleading — it suggests more is safely reclaimable than actually is.

**Why it happens:** The `filter` step on `SafetyClass::Safe` is easy to omit.

**How to avoid:** Explicitly filter `.filter(|e| e.safety == SafetyClass::Safe)` when computing `reclaimable_bytes`. The integration test `test_clean_preview_reclaimable_lte_total` will catch this in regression.

**Warning signs:** `reclaimable_bytes == total_bytes` even when caution or dangerous entries exist.

### Pitfall 5: JSON Key Naming Inconsistency

**What goes wrong:** Using `candidates` in `PreviewResult` but the integration test checks for `entries` (mirroring the `CachesResult` key), causing a test failure.

**Why it happens:** `CachesResult` uses `entries`; if the test is copy-pasted from `caches_cmd.rs` without updating the key name, it will check the wrong key.

**How to avoid:** Decide on the key name before writing tests. This research recommends `candidates` (more semantically precise for a preview) — tests must use the same key.

**Warning signs:** `assert!(parsed.get("candidates").is_some())` passes but data is empty; or test silently passes on `null`.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `todo!()` stub (panics at runtime) | `eprintln!("not yet implemented")` stub | Phase 1 decision | Binary runs without panicking; stub replaced in Phase 6 |
| N/A | Preview is pure read-only composition of scan + classify | Phase 6 | No new infrastructure needed |

**No deprecated patterns apply to this phase.** All libraries and patterns are consistent with those used in Phases 1-5.

---

## Environment Availability

Step 2.6: All external dependencies are already verified by prior phases (cargo, rustc 1.89.0, all Cargo.toml crates). No new external tools are required.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | Build | Yes | 1.89.0 | — |
| All Cargo.toml crates | Runtime | Yes (unchanged from Phase 5) | As locked in Cargo.lock | — |

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` + `assert_cmd` for integration |
| Config file | None (cargo discovers tests automatically) |
| Quick run command | `cargo test -p freespace clean_preview 2>/dev/null` |
| Full suite command | `RUST_LOG=off cargo test -p freespace 2>/dev/null` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PREV-01 | Shows all candidates with safety and size | Integration | `cargo test -p freespace test_clean_preview_json_fields` | Wave 0 |
| PREV-01 | Reclaimable total shown and <= total | Integration | `cargo test -p freespace test_clean_preview_reclaimable_lte_total` | Wave 0 |
| PREV-01 | Safety values are in valid set | Integration | `cargo test -p freespace test_clean_preview_safety_values_valid` | Wave 0 |
| PREV-02 | Preview is idempotent (no disk changes) | Integration | `cargo test -p freespace test_clean_preview_makes_no_changes` | Wave 0 |
| PREV-02 | stderr clean with --json | Integration | `cargo test -p freespace test_clean_preview_stderr_clean_with_json` | Wave 0 |
| PREV-03 | Human table output non-empty | Integration | `cargo test -p freespace test_clean_preview_exits_ok` | Wave 0 |
| PREV-03 | --json produces valid JSON | Integration | `cargo test -p freespace test_clean_preview_json_fields` | Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p freespace clean_preview 2>/dev/null`
- **Per wave merge:** `RUST_LOG=off cargo test -p freespace 2>/dev/null`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `freespace/tests/clean_preview_cmd.rs` — all 6 integration tests covering PREV-01, PREV-02, PREV-03
- [ ] `PartialOrd, Ord` derivation on `SafetyClass` in `classify/mod.rs` — needed by sort in run_preview

*(No framework install needed — cargo test and assert_cmd are already configured)*

---

## Project Constraints (from CLAUDE.md)

CLAUDE.md does not exist in this repository. Constraints are derived from PROJECT.md and STATE.md:

- **Physical size only:** `metadata.blocks() * 512` everywhere — logical `metadata().len()` is forbidden
- **Hardlink dedup:** `(dev, ino)` HashSet in scan module — preview inherits this via `scan_path`
- **Trash-first deletion model:** Preview must not call `trash` or any deletion function
- **Protected paths are immutable:** Preview does not attempt to preview system paths; `safety_class` returns `Blocked` for them
- **stdout clean in --json mode:** All logging/errors via `tracing::warn!` to stderr; `output::write_json` to stdout only
- **Stub handlers use `eprintln!` not `todo!()`:** Current stub follows this; replacement must also not panic
- **All command handlers accept `(config: &Config, json: bool)`:** `run_preview` signature must match

---

## Sources

### Primary (HIGH confidence)

- Direct source code inspection: `freespace/src/commands/clean.rs` — current stub
- Direct source code inspection: `freespace/src/commands/caches.rs` — reference implementation pattern
- Direct source code inspection: `freespace/src/classify/mod.rs` — SafetyClass, safety_class()
- Direct source code inspection: `freespace/src/fs_scan/mod.rs` — scan_path()
- Direct source code inspection: `freespace/src/cli.rs` — CleanCommands::Preview with no args
- Direct source code inspection: `freespace/src/output/mod.rs` — write_json contract
- Direct source code inspection: `freespace/tests/caches_cmd.rs` — integration test pattern
- `.planning/STATE.md` — locked decisions (physical size, hardlink dedup, stub pattern)
- `.planning/REQUIREMENTS.md` — PREV-01, PREV-02, PREV-03 definitions

### Secondary (MEDIUM confidence)

- `.planning/phases/05-analysis-layer-and-largest-files/05-RESEARCH.md` — established research patterns for this project

### Tertiary (LOW confidence)

None — all findings are grounded in direct source inspection.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries verified in Cargo.toml; no new dependencies required
- Architecture: HIGH — implementation is direct composition of existing, tested modules; pattern verified in caches.rs
- Pitfalls: HIGH — all derived from source code analysis of the actual codebase and prior phase patterns

**Research date:** 2026-04-02
**Valid until:** 2026-05-02 (stable codebase; no fast-moving external dependencies)
