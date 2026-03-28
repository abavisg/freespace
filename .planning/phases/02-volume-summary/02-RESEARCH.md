# Phase 2: Volume Summary - Research

**Researched:** 2026-03-28
**Domain:** Rust disk info (sysinfo Disks API), human-readable formatting (bytesize), table rendering (comfy-table), JSON output (serde_json)
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Use `sysinfo` crate `Disks` API for volume enumeration — it's already in Cargo.toml
- If `sysinfo` doesn't expose filesystem type strings on macOS, fall back to `statvfs` via `nix` crate or just omit filesystem type
- Output: human-readable `comfy-table` table by default; clean JSON with `--json`
- JSON must go to stdout only; logs/errors to stderr
- The `platform::macos` module already exists — add volume logic there or in `commands/summary.rs`
- `sysinfo` Disks API is blocking — do not call in a tight loop
- Human-readable sizes: use appropriate units (B, KB, MB, GB, TB)

### Claude's Discretion
All implementation choices are at Claude's discretion within the constraints above.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SUMM-01 | `freespace summary` lists all mounted volumes with mount point, total bytes, used bytes, and available bytes | `Disks::new_with_refreshed_list()` returns all mounted volumes; `Disk::mount_point()`, `total_space()`, `available_space()` provide the needed fields; used = total - available |
| SUMM-02 | Summary output is human-readable table by default and clean JSON with `--json` | `comfy-table` renders the table; `output::write_json()` already handles the JSON path; `bytesize::ByteSize::b(n).to_string()` formats byte counts |
</phase_requirements>

---

## Summary

Phase 2 implements `freespace summary` — the first command with real disk logic. The task is straightforward: enumerate mounted volumes via `sysinfo::Disks`, compute used space as `total_space() - available_space()`, format numbers with `bytesize`, render with `comfy-table` for the default path, and serialize with `serde_json` via the existing `output::write_json()` for `--json`. All dependencies are already in `Cargo.toml`; no manifest changes are needed.

The key fact that changes the implementation approach: `sysinfo` does not expose a `used_space()` method directly. Used bytes must be derived as `total_space() - available_space()`. This is standard and correct — both methods are present on `sysinfo::Disk` in version 0.38.

The output module already has `write_json<T: serde::Serialize>()`. To use it for volume data, define a `VolumeInfo` struct with `#[derive(Serialize)]` and pass a `Vec<VolumeInfo>` to `write_json`. No changes to `output/mod.rs` are needed.

**Primary recommendation:** Define `VolumeInfo` in `platform::macos`, implement a `list_volumes()` function there, and keep `commands/summary.rs` as a thin dispatch layer. This keeps disk logic platform-isolated for future Linux/Windows support.

---

## Standard Stack

### Core (all already in Cargo.toml — no changes needed)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| sysinfo | 0.38 | Disk enumeration: mount point, total, available | Already in project; provides the exact three fields needed |
| comfy-table | 7.2 | Terminal table with column alignment and borders | Already in project; the declared table library for all tabular output |
| bytesize | 2.3 | Human-readable byte formatting | Already in project; single call produces "1.5 GB" style output |
| serde / serde_json | 1.0 | JSON serialization | Already in project; `write_json()` already exists in `output/mod.rs` |

### No New Dependencies

This phase requires zero `Cargo.toml` changes. Every library needed is already declared.

---

## Architecture Patterns

### Recommended File Changes

```
src/
├── platform/
│   └── macos.rs         # ADD: VolumeInfo struct + list_volumes() fn
├── commands/
│   └── summary.rs       # REPLACE stub with real dispatch logic
└── output/
    └── mod.rs           # NO CHANGES needed
```

### Pattern 1: VolumeInfo Struct in platform::macos

**What:** A plain struct with `#[derive(Serialize)]` that holds the four fields: mount point (String), total bytes (u64), used bytes (u64), available bytes (u64).

**When to use:** Define here so disk logic stays platform-isolated. Any future platform module (Linux) would define its own `list_volumes()` with the same signature.

```rust
// Source: sysinfo 0.38 docs https://docs.rs/sysinfo/0.38.0/sysinfo/struct.Disk.html
use serde::Serialize;

#[derive(Serialize)]
pub struct VolumeInfo {
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

#[cfg(target_os = "macos")]
pub fn list_volumes() -> Vec<VolumeInfo> {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    disks.list()
        .iter()
        .map(|disk| {
            let total = disk.total_space();
            let available = disk.available_space();
            VolumeInfo {
                mount_point: disk.mount_point().to_string_lossy().into_owned(),
                total_bytes: total,
                used_bytes: total.saturating_sub(available),
                available_bytes: available,
            }
        })
        .collect()
}
```

**Key points:**
- `Disks::new_with_refreshed_list()` is the one-shot constructor — creates and populates in one call.
- `disk.mount_point()` returns `&Path`; use `.to_string_lossy().into_owned()` for owned String.
- `disk.name()` returns `&OsStr` (disk identifier like "disk1s1") — useful for display but not a required field per SUMM-01.
- `used_bytes` = `total - available`. `sysinfo` has no `used_space()` method. Use `saturating_sub` to guard against any edge case where available > total.
- `Disks` is not `Copy` or `Clone` — iterate `.list()` to get `&[Disk]`.

### Pattern 2: Table Rendering in commands/summary.rs

**What:** Build a `comfy_table::Table`, set headers, add one row per volume with human-formatted byte strings.

**When to use:** Default (non-JSON) output path.

```rust
// Source: comfy-table 7.2 docs https://docs.rs/comfy-table/7.2.2/comfy_table/
use comfy_table::Table;
use bytesize::ByteSize;

fn render_table(volumes: &[VolumeInfo]) {
    let mut table = Table::new();
    table.set_header(vec!["Mount Point", "Total", "Used", "Available"]);
    for v in volumes {
        table.add_row(vec![
            v.mount_point.clone(),
            ByteSize::b(v.total_bytes).to_string(),
            ByteSize::b(v.used_bytes).to_string(),
            ByteSize::b(v.available_bytes).to_string(),
        ]);
    }
    println!("{table}");
}
```

### Pattern 3: JSON Output in commands/summary.rs

**What:** Call the existing `output::write_json()` with `&Vec<VolumeInfo>`. No new code in `output/mod.rs`.

```rust
// Reuses existing output::write_json() from src/output/mod.rs
use crate::output;
use crate::platform::macos;

pub fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    let _ = config; // not used by summary
    let volumes = macos::list_volumes();
    if json {
        output::write_json(&volumes)?;
    } else {
        render_table(&volumes);
    }
    Ok(())
}
```

**Key constraint:** `eprintln!` for errors only. `println!` for table. `write_json` for JSON. Never mix paths.

### Pattern 4: bytesize Formatting

**What:** `ByteSize::b(n).to_string()` — simplest form. Produces output like "1.5 GB", "512 MB", "4.0 TB".

```rust
// Source: https://docs.rs/bytesize/2.3.1/bytesize/struct.ByteSize.html
use bytesize::ByteSize;

let formatted = ByteSize::b(1_500_000_000u64).to_string(); // "1.5 GB"
```

Note: `ByteSize::b(n)` takes `u64`. `total_space()` and `available_space()` both return `u64`, so no casting needed.

### Anti-Patterns to Avoid

- **Hand-rolling byte formatting:** Do not write `format!("{:.1} GB", bytes as f64 / 1e9)`. Edge cases at unit boundaries are subtle. Use `ByteSize::b(n).to_string()`.
- **Calling `Disks::new_with_refreshed_list()` per row or in a loop:** Call once, store result, iterate the list. The call hits `statvfs` syscalls for each disk.
- **Writing `used_space()` — it doesn't exist on `sysinfo::Disk` 0.38.** Always derive as `total.saturating_sub(available)`.
- **Putting disk logic in `commands/summary.rs` directly:** Keep it in `platform::macos` behind the cfg gate.
- **Emitting any output in the JSON path before `write_json`:** Not even a `tracing::info!` to stdout. All tracing goes to stderr (already configured in `init_logging()`).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Human-readable byte sizes | `format!("{:.1} GB", ...)` | `bytesize::ByteSize::b(n).to_string()` | Unit boundary rounding, correct SI/IEC labeling, consistent precision |
| Terminal table layout | Custom column-width alignment | `comfy_table::Table` | Unicode-aware, handles terminal width, already in deps |
| JSON stdout routing | Manual `serde_json::to_string` + `println!` | `output::write_json()` | Already established pattern in the codebase; newline handling included |
| Disk enumeration | `libc::statvfs` directly | `sysinfo::Disks` | Already wraps statvfs correctly; handles macOS quirks |

**Key insight:** Every tool needed for this phase is already wired and tested in Phase 1. Phase 2 is about connecting them, not building new infrastructure.

---

## Common Pitfalls

### Pitfall 1: No used_space() Method on sysinfo::Disk

**What goes wrong:** Searching for `disk.used_space()` or `disk.used()` — these do not exist in sysinfo 0.38.
**Why it happens:** sysinfo exposes `total_space()` and `available_space()` from the OS. "Used" is a derived value.
**How to avoid:** Always compute `used = total.saturating_sub(available)`. Use `saturating_sub` not `-` to guard against any edge where `available > total` (theoretically possible on some networked/fuse filesystems).
**Warning signs:** Compile error `no method named used_space found for struct Disk`.

### Pitfall 2: mount_point() Returns &Path, Not &str

**What goes wrong:** Calling `.mount_point().to_str().unwrap()` panics on non-UTF-8 mount points (rare but possible).
**Why it happens:** macOS paths are technically arbitrary bytes.
**How to avoid:** Use `.to_string_lossy().into_owned()` which replaces invalid UTF-8 with replacement chars rather than panicking.
**Warning signs:** `unwrap()` on `Option<&str>` from `to_str()`.

### Pitfall 3: Disks List Includes Virtual/Pseudo Filesystems

**What goes wrong:** `Disks::new_with_refreshed_list()` on macOS may include volumes like `/dev` or `/private/var/vm`. These clutter the output.
**Why it happens:** sysinfo enumerates all mounts from the OS mount table.
**How to avoid:** For Phase 2, accept all volumes — this is accurate to SUMM-01 which says "all mounted volumes". If filtering is needed later, check `disk.file_system()` for `apfs`, `hfs`, `exfat`, `msdos` to include only "real" volumes; filter out `devfs`, `autofs`, etc.
**Warning signs:** Output shows `/dev` with 0 bytes total.

### Pitfall 4: JSON Has Trailing Newline from write_json

**What goes wrong:** `write_json` calls `serde_json::to_writer` then `writeln!`. This is intentional (POSIX: output ends in newline) but scripts piping to `jq` sometimes care.
**Why it happens:** The function is designed this way from Phase 1.
**How to avoid:** This is correct behavior. Do not change `write_json`. The trailing newline is correct for POSIX tools and `jq` handles it fine.
**Warning signs:** Not a bug — just be aware.

### Pitfall 5: bytesize 2.x API Change from 1.x

**What goes wrong:** Looking up bytesize 1.x examples (most web search results) which use `ByteSize::b(n).to_string()` with a different Display output format. In 2.x, `display()` is a new fluent builder for SI/IEC selection.
**Why it happens:** bytesize 2.x added `display()` as an alternative. The plain `to_string()` still works and produces decimal (SI) formatted output.
**How to avoid:** Use `ByteSize::b(n).to_string()` — this works in both 1.x and 2.x. If you want IEC (GiB, MiB), use `.display().iec().to_string()`.
**Warning signs:** Compiler errors about `.display()` not existing — that's a 1.x environment.

---

## Code Examples

### Full list_volumes() implementation

```rust
// Source: https://docs.rs/sysinfo/0.38.0/sysinfo/struct.Disks.html
// Source: https://docs.rs/sysinfo/0.38.0/sysinfo/struct.Disk.html
use sysinfo::Disks;

#[cfg(target_os = "macos")]
pub fn list_volumes() -> Vec<VolumeInfo> {
    let disks = Disks::new_with_refreshed_list();
    disks.list()
        .iter()
        .map(|disk| {
            let total = disk.total_space();
            let available = disk.available_space();
            VolumeInfo {
                mount_point: disk.mount_point().to_string_lossy().into_owned(),
                total_bytes: total,
                used_bytes: total.saturating_sub(available),
                available_bytes: available,
            }
        })
        .collect()
}
```

### Table render with comfy-table and bytesize

```rust
// Source: https://docs.rs/comfy-table/7.2.2/comfy_table/struct.Table.html
// Source: https://docs.rs/bytesize/2.3.1/bytesize/struct.ByteSize.html
use comfy_table::Table;
use bytesize::ByteSize;

fn render_table(volumes: &[VolumeInfo]) {
    let mut table = Table::new();
    table.set_header(vec!["Mount Point", "Total", "Used", "Available"]);
    for v in volumes {
        table.add_row(vec![
            v.mount_point.clone(),
            ByteSize::b(v.total_bytes).to_string(),
            ByteSize::b(v.used_bytes).to_string(),
            ByteSize::b(v.available_bytes).to_string(),
        ]);
    }
    println!("{table}");
}
```

### summary::run() dispatch

```rust
// Pattern: thin command handler, platform logic isolated in platform::macos
use crate::config::schema::Config;
use crate::output;
#[cfg(target_os = "macos")]
use crate::platform::macos;

pub fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    let _ = config;
    #[cfg(target_os = "macos")]
    {
        let volumes = macos::list_volumes();
        if json {
            output::write_json(&volumes)?;
        } else {
            render_table(&volumes);
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("summary: not supported on this platform");
    }
    Ok(())
}
```

### VolumeInfo struct with serde derives

```rust
// Serde derive is already in Cargo.toml via serde = { version = "1.0", features = ["derive"] }
use serde::Serialize;

#[derive(Serialize)]
pub struct VolumeInfo {
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `sys-info` crate for disk info | `sysinfo` 0.38 | ~2021 | sysinfo is actively maintained, cross-platform, has better API |
| bytesize 1.x `ByteSize::b(n).to_string()` | Same call works; 2.x adds `.display().iec()/.si()` fluent API | bytesize 2.0 (2024) | No breaking change; existing code still works |
| `Disks::new()` + manual `refresh()` | `Disks::new_with_refreshed_list()` one-shot | sysinfo 0.30+ | Simpler; no need to call refresh manually for single-shot use |

**Deprecated/outdated:**
- `sys_info` crate: Last released 2020. Do not use — replaced by `sysinfo`.
- `disk.used_space()`: Never existed. Derived as `total.saturating_sub(available)`.

---

## Existing Code Inventory

| File | Status | What Exists | What Changes |
|------|--------|-------------|--------------|
| `src/commands/summary.rs` | Stub | `run(config, json)` returns `eprintln!("not yet implemented")` | Full implementation replacing the stub |
| `src/output/mod.rs` | Complete | `write_json<T: Serialize>()`, `OutputFormat` enum | No changes needed |
| `src/platform/macos.rs` | Partial | `protected_paths()`, `is_protected()` | Add `VolumeInfo` struct + `list_volumes()` |
| `Cargo.toml` | Complete | All needed deps present: sysinfo 0.38, comfy-table 7.2, bytesize 2.3 | No changes needed |

The existing `output::write_json()` writes to `stdout.lock()` and appends a newline. It accepts any `T: serde::Serialize`. Passing `&Vec<VolumeInfo>` (where `VolumeInfo: Serialize`) works directly.

---

## Open Questions

1. **Should pseudo-filesystems (devfs, autofs) be filtered from output?**
   - What we know: SUMM-01 says "all mounted volumes" — literal reading includes them
   - What's unclear: Whether users expect to see `/dev` entries in the table
   - Recommendation: Include all in Phase 2 for spec compliance. If filtering is desired, use `disk.file_system()` comparison in a follow-on; this is easy to add later.

2. **Should `VolumeInfo` include `name` (disk identifier like "disk1s1") or `fs_type` fields?**
   - What we know: SUMM-01 requires only mount_point, total, used, available
   - What's unclear: Whether extra fields in JSON would be useful for downstream consumers
   - Recommendation: Implement exactly SUMM-01's four fields for Phase 2. Adding fields is additive and non-breaking — defer to when a caller needs them.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`#[test]`) + assert_cmd 2.0 |
| Config file | None — standard `cargo test` |
| Quick run command | `cargo test` (runs in ~0.1s when no rebuild needed) |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SUMM-01 | `list_volumes()` returns a non-empty Vec with correct fields | unit | `cargo test platform::macos::tests::list_volumes` | Wave 0 |
| SUMM-01 | `used_bytes` equals `total_bytes - available_bytes` | unit | `cargo test platform::macos::tests::used_bytes_derived` | Wave 0 |
| SUMM-01 | `VolumeInfo` serializes correctly to JSON | unit | `cargo test platform::macos::tests::volume_info_serializes` | Wave 0 |
| SUMM-02 | `freespace summary` exits 0 and prints table to stdout | integration | `cargo test summary_table_output` (assert_cmd) | Wave 0 |
| SUMM-02 | `freespace summary --json` exits 0, stdout is valid JSON, stderr is empty | integration | `cargo test summary_json_output` (assert_cmd) | Wave 0 |
| SUMM-02 | `freespace summary --json` JSON array contains required fields | integration | `cargo test summary_json_fields` (assert_cmd) | Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green (23 existing + new tests) before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] Unit tests in `src/platform/macos.rs` — covers SUMM-01: `list_volumes_returns_nonempty`, `used_bytes_derived_correctly`, `volume_info_serializes_to_json`
- [ ] Integration tests in `tests/summary_cmd.rs` — covers SUMM-02: `summary_table_exits_0`, `summary_json_exits_0`, `summary_json_is_valid`, `summary_json_has_required_fields`

Note on disk enumeration testing: Unit tests for `list_volumes()` call the real `sysinfo::Disks` on the host macOS system. This is appropriate — the root volume `/` is always mounted, so `assert!(!volumes.is_empty())` is deterministic on any macOS host. Tests must run under `#[cfg(target_os = "macos")]`.

---

## Sources

### Primary (HIGH confidence)
- https://docs.rs/sysinfo/0.38.0/sysinfo/struct.Disk.html — `mount_point()`, `total_space()`, `available_space()`, `name()`, `file_system()` return types
- https://docs.rs/sysinfo/0.38.0/sysinfo/struct.Disks.html — `new_with_refreshed_list()` constructor, `.list()` iterator pattern
- https://docs.rs/bytesize/2.3.1/bytesize/struct.ByteSize.html — `ByteSize::b(n)`, `to_string()`, `display().si()/iec()` API
- https://docs.rs/comfy-table/7.2.2/comfy_table/ — `Table::new()`, `set_header()`, `add_row()`, `println!("{table}")` pattern
- https://docs.rs/assert_cmd/2.0.16/assert_cmd/ — `Command::cargo_bin()`, `.assert().success()`, `.stdout()`, `.stderr()` pattern
- Project source: `src/output/mod.rs` — confirmed `write_json<T: Serialize>()` signature and behavior
- Project source: `Cargo.toml` — confirmed all needed deps present (sysinfo 0.38, comfy-table 7.2, bytesize 2.3, assert_cmd 2.0)

### Secondary (MEDIUM confidence)
- https://docs.rs/sysinfo/0.38.0/sysinfo/ — General sysinfo 0.38 module overview; Disks section

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crates already in Cargo.toml, versions confirmed against docs.rs
- Architecture patterns: HIGH — sysinfo Disk API methods verified against official docs; code patterns are direct translations
- Pitfalls: HIGH — `used_space()` absence confirmed by docs.rs; `to_string_lossy()` is standard Rust guidance for OsStr; bytesize 2.x API verified
- Test strategy: HIGH — existing test infra confirmed with `cargo test`; assert_cmd already in dev-dependencies

**Research date:** 2026-03-28
**Valid until:** 2026-06-28 (sysinfo releases frequently; re-verify if upgrading past 0.38)
