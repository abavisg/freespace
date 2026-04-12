# Phase 7: Cleanup Apply — Context

**Gathered:** 2026-04-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 7 delivers `freespace clean apply` — the only phase with irreversible side effects. It moves targeted files to macOS Trash (default) or permanently deletes them (`--force`), enforces protected-path guards unconditionally, writes an audit log, and requires a prior preview session before acting.

After this phase: users can safely reclaim disk space with a clear paper trail and no accidental deletions.

</domain>

<decisions>
## Implementation Decisions

### 1. Network Volume Behavior (SMB/AFP)

**Decision: Warn + skip**

When a candidate path is on a network volume (where `trash-rs` cannot create a volume-local trash), log a warning to stderr ("skipped: network volume — {path}") and continue with local files. Do not abort the entire apply. Do not permanently delete network files.

**Future config option (v2):** `cleanup.network_volume = "skip" | "force_delete"` — allow users who understand the risk to opt into permanent deletion on network volumes via `--force`.

---

### 2. Confirmation Gate

**Decision: Require y/N confirmation before acting**

Before executing any deletions, print a summary:
```
X items, Y total bytes
Proceed? [y/N]
```
Default is N (no action on Enter). User must explicitly type `y` to proceed.

Rationale: Preview is the review step, but apply is irreversible — a confirmation prompt is belt-and-suspenders for power users who may pipe or script commands.

**Future config option (v2):** `cleanup.confirm = true | false` — allow users to disable the prompt for scripted/automated use cases.

---

### 3. Audit Log

**Decision: JSON Lines format, unbounded growth**

- Format: JSON Lines (one JSON object per line) at `~/.local/state/Freespace/cleanup.log`
- Each entry: `{ "timestamp": "<ISO8601>", "path": "<abs path>", "size_bytes": <u64>, "action": "trash" | "delete" | "skip" }`
- Rotation: None — log grows unbounded. User manages it.
- Directory created on first write if it doesn't exist.

**Future config option (v2):** `cleanup.log_rotation = "none" | "size:10MB" | "entries:1000"` — allow users to configure rotation policy.

---

### 4. APPLY-05 Enforcement — Session State File

**Decision: State file gate**

Preview writes a session file at `~/.local/state/Freespace/preview-session.json` containing:
- Candidate list (paths + sizes + safety classifications)
- Timestamp of when preview was run

Apply reads the session file and:
- Refuses with an informative error if the file is missing ("run `freespace clean preview` first")
- Refuses if the session is stale (older than 1 hour) with a message ("preview session expired — run `freespace clean preview` again")
- On success, acts on the candidate list from the session file (not re-derived)

This ensures apply operates on exactly what the user reviewed in preview.

**Future config option (v2):** `cleanup.session_ttl_minutes = 60` — allow users to tune the session expiry window.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/commands/clean.rs` — `run_preview()` (complete), `run_apply()` (stub), `known_cache_dirs()`, `PreviewEntry`, `PreviewResult`
- `src/classify/mod.rs` — `SafetyClass` enum (Safe/Caution/Dangerous/Blocked), `safety_class()`
- `src/platform/macos.rs` — `protected_paths()` for immutable block list
- `src/output/mod.rs` — `write_json()` for JSON output
- `src/config/schema.rs` — `Config` struct passed to all command handlers
- `trash` crate (v5.2) — already in Cargo.toml

### Established Patterns
- `anyhow::Result` in command handlers
- `#[derive(Serialize)]` on data structs for JSON output
- `comfy_table` for table output
- `tracing` for stderr logging
- All errors/logs → stderr; stdout clean when `--json` not set
- Physical size via `st_blocks * 512` (not `metadata().len()`)

### Integration Points
- `run_apply()` in `clean.rs` — expand the stub
- `platform::macos::protected_paths()` — must be checked before every deletion
- Session file written by `run_preview()`, read by `run_apply()`
- Audit log at `~/.local/state/Freespace/cleanup.log` — new file, created on first write

### CLI
- `freespace clean apply` — already wired in `cli.rs` with `--force: bool`
- No changes to CLI struct needed

</code_context>

<canonical_refs>
## Canonical References

- `.planning/REQUIREMENTS.md` — APPLY-01 through APPLY-05
- `.planning/ROADMAP.md` — Phase 7 success criteria
- `.planning/STATE.md` — Accumulated decisions (trash-first model, protected-path immutability)
- `freespace/src/commands/clean.rs` — existing preview implementation and apply stub
- `freespace/src/classify/mod.rs` — SafetyClass enum
- `freespace/src/platform/macos.rs` — protected_paths()
- `freespace/Cargo.toml` — trash crate version

</canonical_refs>

<deferred>
## Deferred Ideas

The following options were surfaced during discussion but deferred to v2 as config settings:

- `cleanup.network_volume = "skip" | "force_delete"` — opt-in permanent delete on SMB/AFP volumes
- `cleanup.confirm = true | false` — disable y/N confirmation prompt for scripted use
- `cleanup.log_rotation = "none" | "size:10MB" | "entries:1000"` — audit log rotation policy
- `cleanup.session_ttl_minutes = 60` — tune how long a preview session stays valid before expiring

</deferred>
