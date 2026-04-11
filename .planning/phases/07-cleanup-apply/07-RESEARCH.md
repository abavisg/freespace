# Phase 7: Cleanup Apply — Research

**Researched:** 2026-04-11
**Domain:** Rust file deletion (trash-rs), session state, audit logging, protected-path enforcement
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

1. **Network Volume Behavior**: Warn + skip. Log warning to stderr ("skipped: network volume — {path}"), continue with local files. Do not abort, do not permanently delete.
2. **Confirmation Gate**: y/N prompt before acting. Print summary "X items, Y total bytes / Proceed? [y/N]". Default N. User must type `y` to proceed.
3. **Audit Log**: JSON Lines at `~/.local/state/Freespace/cleanup.log`. One JSON object per line. Fields: `{ "timestamp": "<ISO8601>", "path": "<abs path>", "size_bytes": <u64>, "action": "trash" | "delete" | "skip" }`. No rotation. Directory created on first write.
4. **APPLY-05 Session Gate**: Preview writes `~/.local/state/Freespace/preview-session.json`. Apply reads it; refuses if missing or older than 1 hour. On success, acts on candidate list from session (not re-derived).

### Claude's Discretion

- Nothing explicitly listed as discretion in CONTEXT.md. Implementation details (error message wording, code structure, internal module organization) are at discretion as long as they satisfy the locked decisions.

### Deferred Ideas (OUT OF SCOPE)

- `cleanup.network_volume = "skip" | "force_delete"` — config option for network volume behavior
- `cleanup.confirm = true | false` — config option to disable confirmation prompt
- `cleanup.log_rotation = "none" | "size:10MB" | "entries:1000"` — log rotation policy
- `cleanup.session_ttl_minutes = 60` — configurable session expiry window

</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| APPLY-01 | `freespace clean apply` moves files to macOS Trash using the `trash` crate | `trash::delete()` API verified — single function call per path. Default method is Finder (supports "Put Back"). NsFileManager method also available. |
| APPLY-02 | Permanent deletion requires explicit `--force` flag; without it, command refuses permanent delete | `--force: bool` already wired in `cli.rs`. Guard: if `!force`, call `trash::delete()`; if `force`, call `std::fs::remove_file()` / `remove_dir_all()`. |
| APPLY-03 | Protected paths blocked and logged — deletion never executes | `platform::macos::is_protected()` already implemented. Call `std::fs::canonicalize()` on each candidate path before the check. |
| APPLY-04 | All actions logged to `~/.local/state/Freespace/cleanup.log` with timestamp, path, size, action | JSON Lines append-open pattern verified. `std::fs::OpenOptions::append(true).create(true)` on the log file. |
| APPLY-05 | Apply cannot run without prior preview pass | Session file at `~/.local/state/Freespace/preview-session.json`. Preview writes it; apply reads + validates TTL. |

</phase_requirements>

---

## Summary

Phase 7 is the only phase with irreversible side effects. The implementation expands `run_apply()` from its current stub in `src/commands/clean.rs` into a complete deletion pipeline: session-gate check, confirmation prompt, per-item protected-path guard, network-volume detection, trash/force-delete dispatch, and JSON Lines audit logging.

All required building blocks are already present: `trash` 5.2.5 is in `Cargo.toml`, `platform::macos::is_protected()` is implemented, `SafetyClass` and `PreviewEntry` exist, and the CLI wires `--force: bool` to `run_apply`. What needs building is the session file protocol (write side in `run_preview`, read/validate side in `run_apply`), the network-volume detector using `sysinfo::Disks`, the confirmation prompt loop, the JSON Lines logger, and the per-item deletion dispatch.

TDD ordering for this phase: write tests for each safety invariant (session gate, protected-path block, force guard) FIRST as failing tests, then add the production code that makes them pass. The integration tests use `assert_cmd` with `.write_stdin()` for confirmation prompt testing, temp directories for actual file deletion, and env-var overrides for session file path isolation between test runs.

**Primary recommendation:** Implement in a strict safety-first order: session gate, then protected-path guard, then force guard, then deletion dispatch, then audit log. Each guard should be independently testable with a unit test before wiring into the pipeline.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `trash` | 5.2.5 (resolved) | Move files to macOS Trash | Already in Cargo.toml; provides `trash::delete()` |
| `serde` + `serde_json` | 1.0 | Session file JSON serialization + JSON Lines audit | Already in Cargo.toml; used everywhere |
| `std::fs` | stdlib | File deletion (`remove_file`, `remove_dir_all`), directory creation, canonicalization | No new dep |
| `std::io` | stdlib | Stdin confirmation prompt, file append | No new dep |
| `dirs` 6.0 | 6.0 | Resolve `~/.local/state/Freespace/` path | Already in Cargo.toml |
| `anyhow` | 1.0 | Error propagation in command handlers | Already in Cargo.toml |
| `sysinfo` | 0.38.4 (resolved) | `Disks::new_with_refreshed_list()` for network volume detection | Already in Cargo.toml, used in `list_volumes()` |
| `tracing` | 0.1 | `tracing::warn!()` for skipped network volumes | Already in Cargo.toml |

### No New Dependencies Required

All required libraries are already in `Cargo.toml`. The one consideration is ISO 8601 timestamps for the audit log: `chrono` is available transitively (via `trash` dev-deps on Linux only) but is NOT a direct dependency. The recommended approach is to use `std::time::SystemTime` with manual UTC formatting — this avoids adding a new dep and produces correct RFC 3339 output.

**Installation:** No new `cargo add` commands needed. All deps are already present.

---

## Architecture Patterns

### Recommended Project Structure

No new source files needed. All changes are within:

```
src/commands/clean.rs     — expand run_apply() stub, add write_preview_session(), add network_volume_detector
src/platform/macos.rs     — no changes needed (is_protected already implemented)
```

New data files (runtime, not source):
```
~/.local/state/Freespace/
├── preview-session.json   — written by run_preview(), read by run_apply()
└── cleanup.log            — appended by run_apply() (JSON Lines)
```

### Pattern 1: Session File Gate

**What:** `run_preview()` serializes candidates + timestamp to JSON; `run_apply()` reads + validates before acting.
**When to use:** Required for APPLY-05. Must run BEFORE the confirmation prompt.

```rust
// Session file structures (in clean.rs)
#[derive(Debug, Serialize, Deserialize)]
struct PreviewSession {
    timestamp: u64,          // seconds since UNIX_EPOCH
    candidates: Vec<PreviewEntry>,
}

// Write side (in run_preview, after current render logic)
fn write_preview_session(candidates: &[PreviewEntry]) -> anyhow::Result<()> {
    let state_dir = state_dir()?;          // ~/.local/state/Freespace/
    std::fs::create_dir_all(&state_dir)?;
    let session = PreviewSession {
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)?.as_secs(),
        candidates: candidates.to_vec(),   // requires PreviewEntry: Clone
    };
    let tmp_path = state_dir.join("preview-session.json.tmp");
    let final_path = state_dir.join("preview-session.json");
    let json = serde_json::to_string_pretty(&session)?;
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, &final_path)?;   // atomic on same filesystem
    Ok(())
}

// Read side (in run_apply, first operation)
fn load_preview_session() -> anyhow::Result<PreviewSession> {
    let path = state_dir()?.join("preview-session.json");
    if !path.exists() {
        anyhow::bail!("No preview session found. Run `freespace clean preview` first.");
    }
    let json = std::fs::read_to_string(&path)?;
    let session: PreviewSession = serde_json::from_str(&json)?;
    let age = SystemTime::now()
        .duration_since(UNIX_EPOCH)?.as_secs()
        .saturating_sub(session.timestamp);
    if age > 3600 {
        anyhow::bail!(
            "Preview session expired ({} minutes ago). Run `freespace clean preview` again.",
            age / 60
        );
    }
    Ok(session)
}
```

**Source:** Verified by reading `src/commands/clean.rs` — `PreviewEntry` and `PreviewResult` already defined. `write_preview_session` needs to be added to `run_preview`.

**Important:** `PreviewEntry` needs `#[derive(Clone, Deserialize)]` added to support session serialization round-trip.

### Pattern 2: Network Volume Detection

**What:** Build a set of known-network mount points at the start of `run_apply`, check each candidate path against it.
**When to use:** Required for locked decision "warn + skip" on network volumes.

```rust
// Source: sysinfo 0.38.4 Disk API (verified in source)
fn network_mount_points() -> std::collections::HashSet<std::path::PathBuf> {
    use sysinfo::Disks;
    const NETWORK_FS_TYPES: &[&str] = &["smbfs", "afpfs", "nfs", "webdav", "ftpfs", "ftp", "nfs4"];
    let disks = Disks::new_with_refreshed_list();
    disks.list().iter()
        .filter(|d| {
            let fs = d.file_system().to_string_lossy().to_lowercase();
            NETWORK_FS_TYPES.iter().any(|n| fs.as_str() == *n)
        })
        .map(|d| d.mount_point().to_owned())
        .collect()
}

fn is_network_path(path: &Path, network_mounts: &HashSet<PathBuf>) -> bool {
    network_mounts.iter().any(|mp| path.starts_with(mp))
}
```

**Key insight:** `sysinfo::Disk::file_system()` returns the macOS `f_fstypename` value (e.g., `"smbfs"`, `"afpfs"`, `"nfs"`). This is the correct probe point. The `trash` crate does NOT expose a network-volume error variant — detection must happen BEFORE calling `trash::delete()`.

**Source:** Verified in `sysinfo-0.38.4/src/common/disk.rs` and `sysinfo-0.38.4/src/unix/apple/disk.rs` — `file_system()` reads from `statfs.f_fstypename`.

### Pattern 3: Protected Path Guard

**What:** Canonicalize each candidate path, then check against `protected_paths()`.
**When to use:** Required for APPLY-03. Must run even with `--force`. Blocks ALL deletions to protected paths unconditionally.

```rust
// Source: platform/macos.rs (verified — is_protected() already implemented)
fn check_protected(path: &Path, protected: &[PathBuf]) -> anyhow::Result<()> {
    let canonical = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_owned());   // fail-safe: use original path
    if platform::macos::is_protected(&canonical, protected) {
        anyhow::bail!(
            "Blocked: {} is under a protected system path and cannot be deleted.",
            path.display()
        );
    }
    Ok(())
}
```

**Note:** `is_protected()` currently has a dead_code warning. Using it in `run_apply` will resolve this warning.

### Pattern 4: Audit Log — JSON Lines Append

**What:** Open the log file in append mode (create if missing), write one JSON object per line.
**When to use:** Required for APPLY-04. Write AFTER each successful or skipped action.

```rust
fn append_audit_log(log_path: &Path, entry: &AuditEntry) -> anyhow::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let line = serde_json::to_string(entry)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

#[derive(Serialize)]
struct AuditEntry {
    timestamp: String,   // ISO 8601 UTC: "2026-04-11T14:30:00Z"
    path: String,
    size_bytes: u64,
    action: AuditAction,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum AuditAction {
    Trash,
    Delete,
    Skip,
}
```

**Timestamp without chrono:**
```rust
fn utc_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as ISO 8601 UTC from epoch seconds
    let (year, month, day, hour, min, sec) = epoch_to_utc(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hour, min, sec)
}
```

This requires a small `epoch_to_utc()` helper (pure arithmetic — no external dep). Alternatively, add `chrono` to `Cargo.toml` with `chrono = { version = "0.4", features = ["clock"], default-features = false }` for `Utc::now().to_rfc3339()`. **Recommendation: add chrono to Cargo.toml** — it's already a transitive dep, the feature set is minimal, and the API is one line vs. writing epoch arithmetic. The Phase 1 decision was "All Phase 1-8 dependencies in Cargo.toml now" which supports this.

### Pattern 5: Confirmation Prompt

**What:** Read a single line from stdin, proceed only if it equals `"y"`.
**When to use:** Required before any deletion. Skip when `--json` is set (machine-readable mode implies non-interactive).

```rust
fn confirm_prompt(count: usize, total_bytes: u64) -> anyhow::Result<bool> {
    use std::io::{BufRead, Write};
    print!(
        "{} items, {} — Proceed? [y/N] ",
        count,
        bytesize::ByteSize::b(total_bytes)
    );
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim().eq_ignore_ascii_case("y"))
}
```

**JSON mode bypass:** When `json == true`, skip the prompt entirely. JSON output is consumed by scripts; a confirmation prompt would deadlock a piped invocation.

### Pattern 6: Deletion Dispatch

**What:** Per-candidate dispatch: check protected, check network, then trash or force-delete.

```rust
// Conceptual flow per candidate:
for entry in &session.candidates {
    let canonical = canonicalize_or_original(&entry.path);
    
    // 1. Protected check (unconditional)
    if is_protected(&canonical, &protected) {
        tracing::warn!("blocked: protected path — {}", entry.path.display());
        append_audit_log(&log_path, &AuditEntry { action: AuditAction::Skip, ... })?;
        continue;
    }
    
    // 2. Network volume check
    if is_network_path(&canonical, &network_mounts) {
        eprintln!("skipped: network volume — {}", entry.path.display());
        append_audit_log(&log_path, &AuditEntry { action: AuditAction::Skip, ... })?;
        continue;
    }
    
    // 3. Deletion
    if force {
        if entry.path.is_dir() {
            std::fs::remove_dir_all(&entry.path)?;
        } else {
            std::fs::remove_file(&entry.path)?;
        }
        append_audit_log(&log_path, &AuditEntry { action: AuditAction::Delete, ... })?;
    } else {
        trash::delete(&entry.path).map_err(|e| anyhow::anyhow!("{}", e))?;
        append_audit_log(&log_path, &AuditEntry { action: AuditAction::Trash, ... })?;
    }
}
```

### Anti-Patterns to Avoid

- **Re-deriving candidates in apply:** Apply MUST act on the session candidate list, not re-scan `known_cache_dirs()`. The session file is the contract between preview and apply.
- **Skipping canonicalize before protected-path check:** A symlink can bypass a raw `starts_with` check. Always `canonicalize()` before `is_protected()`.
- **Calling `trash::delete_all()` for the whole list:** Prefer per-item calls in a loop so one failure doesn't silently skip remaining items. The `trash::Error` type on macOS has no per-item failure reporting in batch mode.
- **Writing audit log entries for items NOT yet acted on:** Log AFTER the operation, not before. A pre-log entry that records an action that then errors produces a false audit trail.
- **Prompting when `--json` is set:** JSON mode is for scripted/piped use. A blocking stdin read would deadlock the process.
- **Using `trash::delete_all()` with canonicalized paths directly:** The `trash` crate's `delete_all` canonicalizes paths internally — but that's fine. For our flow, we call per-item anyway.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Moving files to macOS Trash | Custom osascript wrapper | `trash::delete()` | trash-rs handles both Finder and NSFileManager methods, escaping, error handling |
| Detecting file system type | Custom `statfs` FFI | `sysinfo::Disk::file_system()` | Already used in the project for `list_volumes()`; wraps `statfs` correctly |
| Canonicalize-with-fallback | Custom symlink resolver | `std::fs::canonicalize()` + `unwrap_or_else` | stdlib; pattern already established in `protected_paths()` |
| Atomic session file write | Partial write + corrupt state | write to `.tmp` then `std::fs::rename()` | rename is atomic on same filesystem on macOS |
| JSON serialization | Manual string building | `serde_json::to_string()` | Handles escaping, nesting, types correctly |

**Key insight:** The `trash` crate is the only correct way to interact with macOS Trash — direct filesystem manipulation of `~/.Trash` won't set restore metadata correctly and won't support "Put Back" in Finder.

---

## Common Pitfalls

### Pitfall 1: Symlink Bypass of Protected-Path Check
**What goes wrong:** A path like `/tmp/foo` passes `starts_with("/System")` → false, but canonicalizes to `/private/tmp/foo` which starts with `/private` — a protected path.
**Why it happens:** `/tmp` is a symlink to `/private/tmp` on macOS. Raw `starts_with` checks on unresolved paths are bypassable.
**How to avoid:** Always `std::fs::canonicalize(path)` before calling `is_protected()`. Fall back to original path if canonicalize fails (file may not exist).
**Warning signs:** Tests that only test raw paths pass, but a test with `/tmp/...` path fails the protection check.

### Pitfall 2: Session File Written with Wrong PreviewEntry Type
**What goes wrong:** `PreviewEntry` lacks `#[derive(Clone, Deserialize)]`, so session serialization fails to compile or the deserialized struct doesn't match.
**Why it happens:** `PreviewEntry` was designed for one-way JSON output, not round-trip serialization.
**How to avoid:** Add `#[derive(Clone, Deserialize)]` to `PreviewEntry` when adding session file support.

### Pitfall 3: Audit Log Directory Not Created on First Write
**What goes wrong:** `~/.local/state/Freespace/` doesn't exist on fresh installs; `OpenOptions::open()` fails with `NotFound`.
**Why it happens:** The directory is created by the first `run_apply` call — which never happened before.
**How to avoid:** Call `std::fs::create_dir_all(&log_dir)` before opening the log file. Same pattern as `state_dir` creation for session file.

### Pitfall 4: `trash::delete()` on a Path That No Longer Exists
**What goes wrong:** A candidate in the session was already deleted between preview and apply. `trash::delete()` returns `Error::CouldNotAccess` or `Error::CanonicalizePath`.
**Why it happens:** Session file represents state from up to 1 hour ago. Files can be moved/deleted in the interim.
**How to avoid:** Check `path.exists()` before calling `trash::delete()`. If the path doesn't exist, log a `skip` entry with a note and continue. Don't abort the entire apply.

### Pitfall 5: Confirmation Prompt Hangs in Tests
**What goes wrong:** Integration tests that call `freespace clean apply` without `.write_stdin("y\n")` hang waiting for stdin.
**Why it happens:** The confirmation prompt blocks on `stdin().read_line()`.
**How to avoid:** All integration tests that call `clean apply` must provide stdin via `.write_stdin("y\n")` (assert_cmd supports this). Tests that expect apply to FAIL before reaching the prompt (e.g., session-gate test) don't need stdin.

### Pitfall 6: Session File Path Hardcoded in Tests
**What goes wrong:** Integration tests for apply interact with the developer's real `~/.local/state/Freespace/preview-session.json`, causing false passes or interfering with real state.
**Why it happens:** `dirs::home_dir()` returns the actual home directory during tests.
**How to avoid:** Inject an env variable (e.g., `FREESPACE_STATE_DIR`) that overrides the default state directory path. Tests set this to a temp directory. Production code falls back to `~/.local/state/Freespace/` when the env var is absent.

---

## Code Examples

### trash::delete() — Primary API

```rust
// Source: trash-5.2.5/src/lib.rs — verified
// Convenience function wrapping DEFAULT_TRASH_CTX.delete()
pub fn delete<T: AsRef<Path>>(path: T) -> Result<(), Error>

// Usage:
trash::delete(&entry.path).map_err(|e| anyhow::anyhow!("Trash error: {}", e))?;
```

### trash::Error variants relevant to macOS apply

```rust
// Source: trash-5.2.5/src/lib.rs — verified
pub enum Error {
    Unknown { description: String },   // NSFileManager trashItemAtURL failure
    Os { code: i32, description: String },  // osascript exit code failure
    TargetedRoot,                           // tried to delete /
    CouldNotAccess { target: String },      // path doesn't exist or no permission
    CanonicalizePath { original: PathBuf }, // path parent doesn't exist
    // ... restore-only variants not relevant here
}
```

### sysinfo network volume detection

```rust
// Source: sysinfo-0.38.4/src/common/disk.rs — file_system() returns f_fstypename
use sysinfo::Disks;

let disks = Disks::new_with_refreshed_list();
for disk in disks.list() {
    println!("{:?} -> fs={:?}", disk.mount_point(), disk.file_system());
    // macOS network examples: "smbfs", "afpfs", "nfs", "webdav"
}
```

### Atomic session file write (write + rename)

```rust
// Source: established Rust pattern — POSIX rename() is atomic on same filesystem
let tmp = path.with_extension("json.tmp");
std::fs::write(&tmp, &json_bytes)?;
std::fs::rename(&tmp, &path)?;    // atomic replacement
```

### JSON Lines append

```rust
// Source: std::fs::OpenOptions (stdlib) — verified
use std::io::Write;
let mut file = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(&log_path)?;
writeln!(file, "{}", serde_json::to_string(&entry)?)?;
```

### Stdin confirmation prompt

```rust
// Source: std::io (stdlib)
use std::io::{BufRead, Write};
print!("{} items, {} — Proceed? [y/N] ", count, ByteSize::b(total_bytes));
std::io::stdout().flush()?;
let mut response = String::new();
std::io::stdin().lock().read_line(&mut response)?;
// Proceed only if response.trim() == "y" (case-insensitive)
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `trash-rs` uses Finder method by default | v5+ default is `DeleteMethod::Finder`; `NsFileManager` available as opt-in | trash 5.x | Finder method supports "Put Back"; NsFileManager is faster but has macOS bug with "Put Back" on some systems |
| Manual osascript calls | `trash::delete()` handles AppleScript generation | Phase 1 decision | No hand-rolled shell exec needed |

**Key versioning note:** trash 5.2.5 is the resolved version. The default `DeleteMethod::Finder` produces Trash entries recoverable via "Put Back" in Finder's contextual menu — which directly satisfies success criterion 1 ("files are recoverable from Finder Trash afterward").

---

## Open Questions

1. **`--json` mode and confirmation prompt**
   - What we know: CONTEXT.md requires confirmation prompt. `--json` is for machine use. No explicit guidance on interaction.
   - What's unclear: Should `--json` bypass the prompt (non-interactive) or should json mode require `--force` or a separate flag?
   - Recommendation: Bypass the confirmation prompt when `json == true`. JSON mode is consumed by scripts; a blocking stdin read would deadlock piped use. Document this behavior. The prompt is a safety UX feature for interactive use; `--json` implies the caller has already decided.

2. **`remove_dir_all` with `--force` on candidate directories**
   - What we know: `PreviewEntry` tracks directories (e.g., `Library/Caches/com.foo`). `--force` means permanent delete.
   - What's unclear: Is the correct behavior `remove_dir_all` (recursive) or `remove_file` (single file only)?
   - Recommendation: Use `path.is_dir()` check: if directory, use `std::fs::remove_dir_all()`; if file, use `std::fs::remove_file()`. Both are stdlib, no additional dep.

3. **FREESPACE_STATE_DIR env variable override for tests**
   - What we know: Integration tests cannot use the real `~/.local/state/Freespace/` without contaminating developer state.
   - What's unclear: Is there a precedent in the codebase for env-var-based path override?
   - Recommendation: Add `std::env::var("FREESPACE_STATE_DIR").ok()` fallback in the `state_dir()` helper. Tests set this env var via `.env("FREESPACE_STATE_DIR", tmp_dir)`. This is a test-only escape hatch with zero production behavior change when the var is unset.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust / Cargo | Build | Yes | 1.89.0 | — |
| `trash` crate 5.2.5 | APPLY-01 | Yes (Cargo.toml) | 5.2.5 | — |
| `sysinfo` crate 0.38.4 | Network vol detection | Yes (Cargo.toml) | 0.38.4 | — |
| `serde_json` | Session file + audit log | Yes (Cargo.toml) | 1.0 | — |
| `dirs` 6.0 | State dir resolution | Yes (Cargo.toml) | 6.0 | — |
| macOS Finder / osascript | `trash::delete()` internals | Yes (macOS 26.3.1) | — | NsFileManager method |
| `chrono` (optional) | ISO 8601 timestamps | Transitive only | 0.4.44 | `std::time::SystemTime` + manual format |

**Missing dependencies with no fallback:** None — all required libraries are already in Cargo.toml.

**Missing dependencies with fallback:**
- `chrono`: Transitive dep, not in Cargo.toml directly. Either add it (`chrono = { version = "0.4", features = ["clock"], default-features = false }`) or use manual timestamp formatting. Both approaches produce correct output. Adding chrono is recommended for clarity.

---

## Validation Architecture

`nyquist_validation` is enabled in `.planning/config.json`.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `assert_cmd` for integration |
| Config file | `Cargo.toml` `[dev-dependencies]` (assert_cmd 2.0, tempfile 3.27) |
| Quick run command | `cargo test --test clean_apply_cmd` |
| Full suite command | `cargo test` |

### TDD Order — Write Tests BEFORE Implementation

The following tests should be written as FAILING tests first, then the implementation code added to make them pass.

**Wave 1 — Safety invariant tests (write first, implement after):**

1. `test_apply_no_session_fails` — APPLY-05: apply without session file exits non-zero with message containing "preview"
2. `test_apply_expired_session_fails` — APPLY-05: apply with session older than 3600s exits non-zero with message containing "expired"
3. `test_apply_protected_path_never_deletes` — APPLY-03: file under `/private/tmp` is never deleted even with `--force`
4. `test_apply_no_force_refuses_permanent_delete` — APPLY-02: without `--force`, permanent delete does not occur

**Wave 2 — Core behavior tests (write after wave 1 passes):**

5. `test_apply_trash_moves_file` — APPLY-01: file is in Trash after apply (verify original path gone, Trash exists)
6. `test_apply_force_permanently_deletes` — APPLY-02: `--force` removes file, file is not in Trash
7. `test_apply_audit_log_written` — APPLY-04: log file created, valid JSON Lines, correct fields
8. `test_apply_network_volume_skipped` — APPLY-03 (network): network-mount paths logged as skip

**Wave 3 — Edge case tests:**

9. `test_apply_confirmation_default_n_aborts` — Confirm with empty input (Enter) aborts, no files deleted
10. `test_apply_json_mode_skips_prompt` — `--json` bypasses confirmation, proceeds (or errors) without blocking
11. `test_apply_missing_file_skipped_gracefully` — File in session no longer exists at apply time; apply continues
12. `test_apply_audit_log_append_multiple_runs` — Two apply runs; log grows, second entries appended after first

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| APPLY-01 | trash::delete() moves to Trash | integration | `cargo test --test clean_apply_cmd test_apply_trash_moves_file` | No — Wave 0 |
| APPLY-02 | --force guard: without flag, no permanent delete | integration | `cargo test --test clean_apply_cmd test_apply_no_force_refuses_permanent_delete` | No — Wave 0 |
| APPLY-02 | --force: permanent deletion occurs | integration | `cargo test --test clean_apply_cmd test_apply_force_permanently_deletes` | No — Wave 0 |
| APPLY-03 | Protected path blocked unconditionally | integration | `cargo test --test clean_apply_cmd test_apply_protected_path_never_deletes` | No — Wave 0 |
| APPLY-03 | Network volume skipped | integration | `cargo test --test clean_apply_cmd test_apply_network_volume_skipped` | No — Wave 0 |
| APPLY-04 | Audit log written with correct fields | integration | `cargo test --test clean_apply_cmd test_apply_audit_log_written` | No — Wave 0 |
| APPLY-05 | Session gate: no session → error | integration | `cargo test --test clean_apply_cmd test_apply_no_session_fails` | No — Wave 0 |
| APPLY-05 | Session gate: expired session → error | integration | `cargo test --test clean_apply_cmd test_apply_expired_session_fails` | No — Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test --test clean_apply_cmd`
- **Per wave merge:** `cargo test`
- **Phase gate:** Full suite green (`cargo test`) before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `freespace/tests/clean_apply_cmd.rs` — all 12 tests listed above
- [ ] `FREESPACE_STATE_DIR` env var support in `state_dir()` helper — required for test isolation

*(No new framework install needed — assert_cmd and tempfile already in `[dev-dependencies]`)*

### Testing Constraints and Patterns

**APPLY-01 (trash) test strategy:** Call `freespace clean apply` with `.write_stdin("y\n")`, verify the source path no longer exists. Do NOT verify the Trash contains the file (Trash location is internal macOS state; verifying it requires scanning `~/.Trash` which is fragile across macOS versions). The absence of the original path is the correct assertion.

**APPLY-03 (protected path) test strategy:** Cannot actually create files at `/System/...` in tests. Instead, write a unit test in `clean.rs` that calls the internal `is_protected()` check with a mock path list, asserting the protected check fires. For the integration test, use `FREESPACE_STATE_DIR` to inject a session with a candidate path under `/private/tmp` — then verify the file was NOT deleted.

**APPLY-05 (session gate) test strategy:** Do NOT write `preview-session.json` before running apply (or write one with `timestamp = 0`). Assert exit code is non-zero and stderr contains "preview" or "expired".

**Network volume test strategy:** No CI environment has actual SMB mounts. Test the `is_network_path()` helper function as a unit test by constructing a mock `HashSet<PathBuf>` of "network" mount points and calling the helper. Integration test of network-path skip logic uses mock session data with a path under a fake network mount (which will either be skipped by the network check or fail the trash call — either way the audit log should show "skip").

---

## Sources

### Primary (HIGH confidence)

- `trash-5.2.5/src/lib.rs` — Function signatures for `trash::delete()`, `trash::delete_all()`, `Error` enum variants, internal `canonicalize_paths` logic
- `trash-5.2.5/src/macos/mod.rs` — macOS implementation: `DeleteMethod` enum (Finder vs NsFileManager), `delete_using_finder`, `delete_using_file_mgr`
- `sysinfo-0.38.4/src/common/disk.rs` — `Disk::file_system()`, `Disk::mount_point()` public API
- `sysinfo-0.38.4/src/unix/apple/disk.rs` — macOS `file_system()` reads from `statfs.f_fstypename`
- `freespace/src/commands/clean.rs` — existing `PreviewEntry`, `PreviewResult`, `run_preview()` implementation, `run_apply()` stub
- `freespace/src/platform/macos.rs` — `protected_paths()`, `is_protected()` implementations
- `freespace/src/classify/mod.rs` — `SafetyClass` enum with `Ord` derived
- `freespace/Cargo.toml` — confirmed `trash = "5.2"`, `sysinfo = "0.38"`, `dirs = "6.0"` all present
- `assert_cmd-2.2.0/src/cmd.rs` — `.write_stdin()` method confirmed available for stdin injection in tests
- `macOS man statfs(2)` — `f_fstypename`, `MNT_LOCAL` flag documentation; network FS type names

### Secondary (MEDIUM confidence)

- macOS filesystem type names (`"smbfs"`, `"afpfs"`, `"nfs"`) — based on macOS documentation and sysinfo source code, verified by grep through sysinfo codebase; the names "smbfs" and "afpfs" are standard macOS POSIX type names unchanged since OS X 10.6

### Tertiary (LOW confidence)

- trash::Error behavior for network volumes — not explicitly documented; inferred from source that there is no specific network-volume error variant, meaning detection must be pre-deletion. The `Error::Unknown` or `Error::Os` variants would be returned on NSFileManager failure for network paths, but these are generic. Pre-detection via sysinfo is the correct approach.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all deps verified in Cargo.lock/metadata and source
- Architecture: HIGH — all patterns verified against actual source code of trash-rs, sysinfo, existing clean.rs
- Pitfalls: HIGH — symlink bypass and session deserialization issues verified from source; stdin hang is observable behavior
- Network volume detection: MEDIUM — fs type names verified in sysinfo source, but behavior with edge-case network FS types (WebDAV over HTTPS, etc.) is not fully enumerated

**Research date:** 2026-04-11
**Valid until:** 2026-05-11 (stable ecosystem — trash-rs and sysinfo APIs are stable)
