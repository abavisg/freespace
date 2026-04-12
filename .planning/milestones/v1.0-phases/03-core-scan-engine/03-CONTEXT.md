# Phase 3: Core Scan Engine - Context

**Gathered:** 2026-03-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 3 delivers `freespace scan <path>` — the streaming directory traversal engine that all downstream commands depend on. After this phase: the scanner reports total size, file count, directory count, and the top-N largest files and directories for any given path. It handles hardlink deduplication, physical sparse-file sizes, TCC permission errors, broken symlinks, and mid-scan deletions without crashing. This is the highest-risk phase — five of the eight critical pitfalls surface here.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All implementation choices are at Claude's discretion. Key constraints from research and PITFALLS.md:

**Safety-critical (from research):**
- Use `walkdir` 2.5 for streaming traversal — iterator-based, memory proportional to tree depth not file count
- Hardlink deduplication via `(dev, ino)` HashSet — use `std::os::unix::fs::MetadataExt` for ino() and dev()
- Physical size via `metadata.blocks() * 512` (NOT `metadata.len()`) for sparse file correctness
- TCC/permission denied errors: count skipped paths, surface in output, do NOT crash
- Broken symlinks and mid-scan deletions: log to stderr, continue scan, do NOT crash
- `walkdir::WalkDir::new(path).follow_links(false)` — never follow symlinks (prevents loops)

**Architecture:**
- `fs_scan` module: `src/fs_scan/mod.rs` — streaming iterator, no loading full tree into memory
- `analyze` module: `src/analyze/mod.rs` — top-N aggregation using BinaryHeap (added in Phase 5, stub only here)
- `commands/scan.rs` — thin dispatch layer, already exists as stub

**Output:**
- Human-readable table by default (comfy-table)
- `--json` produces clean JSON on stdout
- Skipped paths count shown in output

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/output/mod.rs` — write_json() and table output already working
- `src/commands/scan.rs` — stub exists, needs real implementation
- `src/platform/macos.rs` — protected_paths() available for path checking
- `walkdir = "2.5"` already in Cargo.toml

### Established Patterns
- anyhow::Result in command handlers
- `#[cfg(target_os = "macos")]` for platform-specific code
- tracing macros for stderr logging
- comfy-table for table output

### Integration Points
- `src/commands/scan.rs` → `src/fs_scan/mod.rs` for traversal
- `src/fs_scan/mod.rs` → `src/output/mod.rs` for rendering

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. The PITFALLS.md must be consulted during planning — it has specific guidance on hardlink detection, sparse files, TCC handling, and symlink protection.

</specifics>

<deferred>
## Deferred Ideas

- jwalk parallel traversal (deferred to post-MVP optimization)
- Progress bar for long-running scans (v2)

</deferred>
