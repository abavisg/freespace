# Project Research Summary

**Project:** Freespace
**Domain:** Rust CLI disk inspection and cleanup utility (macOS)
**Researched:** 2026-03-28
**Confidence:** HIGH

## Executive Summary

Freespace is a macOS-native disk inspection and cleanup CLI tool written in Rust. The field is occupied by established utilities — ncdu (interactive TUI), dust (visual tree output), duf (volume summary), and diskus (fast total) — but none of them offer semantic file classification, safety-tiered cleanup candidates, or an enforced Inspect-to-Clean pipeline. The recommended approach is to build a pipeline-first, JSON-scriptable CLI that treats cleanup as a last step that can only be reached after scan and classification have run — a safety guarantee that no competitor provides.

The stack is well-determined: clap 4.x for subcommands, walkdir for streaming traversal, serde/serde_json/toml for serialization, trash 5.x for macOS Trash integration, thiserror + anyhow for layered error handling, and sysinfo for volume metadata. All versions are verified against crates.io as of 2026-03-28. The architecture follows a strict left-to-right pipeline — fs_scan produces a DirEntry stream, classify assigns category and safety class per entry, analyze aggregates into ScanResult, and cleanup requires both classify and a user-confirmed preview before any deletion executes. This ordering is enforced at the module boundary level, not by runtime flags.

The most significant risks are macOS-specific and non-obvious: APFS clone and sparse-file accounting causes logical sizes to substantially overstate physical disk usage; TCC (Privacy) silently denies access to large subtrees without crashing; hardlinked files will be double-counted unless inode deduplication is applied during traversal; and SIP-protected paths must be blocked via canonicalized path prefix matching, not just literal string checks. These must all be addressed in the scan module before classification, cleanup preview, or cleanup apply are built — the module build order is the key constraint.

## Key Findings

### Recommended Stack

The Rust crate ecosystem has stable, well-maintained answers for every component of this tool. No novel dependencies or unproven libraries are needed. The only version constraint to watch is sysinfo 0.38.x requiring rustc 1.88+, which should be pinned in `rust-toolchain.toml`. The trash crate is the only correct implementation of macOS Trash integration in Rust; there is no alternative.

**Core technologies:**
- clap 4.6: CLI argument parsing and subcommand routing — derive macro API eliminates boilerplate; shell completions are automatic
- walkdir 2.5: Recursive directory traversal — iterator-native, deterministic, handles permission errors without panicking; used internally by ripgrep
- serde + serde_json + toml 1.1: Serialization layer — serde derives apply to all result structs; JSON to stdout and TOML config parsing are both covered
- trash 5.2: macOS Trash integration — only correct implementation; files are recoverable from Finder Trash; must always be called via the crate, never reimplemented
- thiserror 2.0 + anyhow 1.0: Error handling — thiserror for typed domain errors in library modules; anyhow for aggregation at main.rs and command handlers
- sysinfo 0.38: Volume enumeration — Disks API covers mount point, total, and available space; blocking syscall, call on background thread if doing live refresh
- comfy-table 7.2 + owo-colors 4.3: Output rendering — comfy-table for aligned terminal tables; owo-colors for color (NO_COLOR env var supported); both are actively maintained
- indicatif 0.18: Progress output — write to stderr only to preserve JSON stdout contract
- bytesize 2.3: Human-readable byte formatting — consistent across table and JSON output

### Expected Features

The feature dependency graph is linear and dictates implementation order: volume summary is standalone; directory scan is a prerequisite for everything else; classification is a prerequisite for cleanup; cleanup preview is a prerequisite for cleanup apply. Every feature in scope for v1 is a P1.

**Must have (table stakes):**
- `freespace summary` — volume total/used/available; users assume any disk tool shows this
- `freespace scan <path>` — streaming recursive traversal with file and dir counts; baseline for everything downstream
- `freespace largest <path>` — top-N files and directories; the single most-used feature across all disk tools
- Human-readable sizes, graceful permission error handling, progress indicator, exclusion support, broken symlink tolerance

**Should have (competitive differentiators — also v1):**
- `freespace categories <path>` — 14-category semantic classification; no competitor does this; primary differentiator
- Safety classification system (safe/caution/dangerous/blocked) — no competitor has safety tiers on cleanup candidates
- `freespace caches` — dedicated cache inspection command with safety class per cache
- `freespace hidden <path>` — developer dotfile and hidden directory audit
- `freespace clean preview` — first-class read-only preview before any deletion
- `freespace clean apply` — trash-first deletion; `--force` for permanent; blocked paths enforced
- `--json` on all major commands — enables scripting; must be canonical, not optional
- Cleanup audit log at `~/.local/state/freespace/cleanup.log`
- macOS known-path registry (`platform::macos`) — highest-priority classification signal

**Defer (v1.x):**
- `freespace doctor` — add once core commands are stable; lower urgency than core pipeline

**Defer (v2+):**
- Interactive TUI (ncdu-style), duplicate file detection, AI-assisted classification, Linux platform support, application uninstaller, scheduled cleanup, real-time watch mode

### Architecture Approach

The architecture is a strictly one-directional pipeline: CLI layer dispatches to command handlers, which call into a core pipeline of fs_scan → classify → analyze → cleanup, supported by config, platform::macos, and output modules. No stage calls backwards. The cleanup module cannot be invoked without a classify result, and cleanup apply cannot execute without a preview having been generated — these are module-boundary guarantees, not runtime conventions. All stdout is reserved for structured output (tables or JSON); all progress, warnings, and errors go to stderr.

**Major components:**
1. `fs_scan` — walkdir-based streaming iterator; inode deduplication; error tolerance; never returns a Vec
2. `classify` — path-first, macOS-known-dir second, extension third, unknown fallback; pure function; independently testable
3. `analyze` — fold-based aggregation over classified iterator; bounded BinaryHeap for top-N; no I/O
4. `cleanup` — preview (read-only) separated from apply (side effects); protected-path guard via canonicalized prefix match; trash-first; log appended on every action
5. `platform::macos` — all macOS-specific code (statvfs, known-path map, PROTECTED_PATHS); gated behind `#[cfg(target_os = "macos")]`
6. `output` — single rendering exit point; JSON to stdout, everything else to stderr; enforces the stdout/stderr contract

### Critical Pitfalls

1. **Hardlink double-counting inflates size totals** — track `HashSet<(dev_id, inode)>` during traversal using `MetadataExt::dev()` and `MetadataExt::ino()`; only count bytes for the first occurrence of each `(dev, ino)` pair. Must be correct in fs_scan before any downstream module is trusted.

2. **File size uses logical length instead of allocated blocks** — use `MetadataExt::st_blocks() * 512` as the primary size-on-disk figure, not `metadata().len()`; sparse files (VM images, Docker volumes) report logical sizes orders of magnitude larger than physical allocation. Define `FileEntry.size` as physical bytes from day one.

3. **macOS TCC silently denies access without crashing** — handle `EPERM` by logging to stderr and incrementing a skip counter; report "N directories skipped (permission denied)" in all scan summaries; probe known TCC-protected paths in `freespace doctor`.

4. **SIP-protected paths must be blocked via canonicalized prefix match** — `std::fs::canonicalize()` before every protected-path check; `/etc` resolves to `/private/etc`; block both traversal entry and deletion; compile-time list of protected prefixes, not exact matches.

5. **APFS clone and snapshot space accounting is not what users expect** — logical sizes overstate physical usage on APFS; `statvfs.f_bavail` does not consistently include purgeable space; never promise "you will recover X GB"; always say "up to X GB may be freed."

## Implications for Roadmap

Based on the module build order from ARCHITECTURE.md and the feature dependency graph from FEATURES.md, the following phase structure is recommended. The constraint is architectural: lower phases must be complete and tested before higher phases can be reliable.

### Phase 1: Project Foundation and Safety Scaffolding

**Rationale:** Protected-path constants and the CLI skeleton must exist before any other module is written. Getting these wrong creates security bugs that are expensive to retrofit. The PITFALLS.md maps SIP/protected-path handling to milestone 1 explicitly.
**Delivers:** Compilable binary with clap subcommand routing, protected-path constant list with canonicalization, config loading from `~/.config/freespace/config.toml`, project structure matching the recommended module layout
**Addresses:** CLI skeleton, config system, protected-path scaffolding
**Avoids:** Pitfall 4 (SIP bypass via unresolved symlink) — establish `canonicalize()` + prefix-check pattern before any path is evaluated

### Phase 2: Volume Summary

**Rationale:** Standalone command with no upstream dependencies; validates the platform::macos module (sysinfo + statvfs) before it is needed by more complex commands; fast to ship.
**Delivers:** `freespace summary` showing mounted volumes with total/used/available; purgeable space surfaced separately
**Uses:** sysinfo 0.38, nix/statvfs, platform::macos, comfy-table, serde_json (--json)
**Avoids:** Pitfall 3 (APFS available space misreport) — establish purgeable space distinction early

### Phase 3: Core Scan Engine

**Rationale:** All downstream features depend on this. Must be correct before classification, analysis, or cleanup are built. All scan-layer pitfalls (hardlinks, sparse files, symlink loops, TCC) must be resolved here.
**Delivers:** `freespace scan <path>` with streaming traversal, inode deduplication, physical-size reporting, permission-error tolerance, skip counter, progress indicator
**Implements:** fs_scan module — the foundation of the entire pipeline
**Avoids:** Pitfalls 1 (hardlink double-counting), 2 (logical vs physical size), and the symlink loop hang; establishes EPERM handling pattern used by all later commands

### Phase 4: Classification Engine and Category Commands

**Rationale:** Classification depends on a correct scan engine. The 14-category system and macOS known-path registry are the primary differentiators — they must be solid before any cleanup is built on top of them. Test fixtures for every macOS standard directory are required before shipping.
**Delivers:** classify module (path-first → known-dir → extension → unknown), `freespace categories <path>`, `freespace hidden <path>`, `freespace caches`, macOS known-path registry in platform::macos
**Avoids:** Pitfall 8 (extension-first classification yielding wrong categories for macOS paths)

### Phase 5: Analysis Layer and Largest Files

**Rationale:** analyze module is pure functions over classified iterators — no I/O, easiest to test. Ships `freespace largest` which is the most-used single feature in any disk tool.
**Delivers:** analyze module with fold-based aggregation and bounded BinaryHeap top-N; `freespace largest <path>`
**Uses:** BinaryHeap for O(n log N) top-N rather than full sort

### Phase 6: Cleanup Preview

**Rationale:** Preview must be implemented and validated before apply is built. This is the architectural gate enforced by the pipeline: no side effects until preview is reliable. The cleanup module's preview.rs is read-only by design.
**Delivers:** `freespace clean preview` showing candidates with safety class, estimated reclaimable space, and source volume; staleness timestamp on output
**Avoids:** Pitfall (cleanup running before scan has produced results) — preview requires a valid classified ScanResult

### Phase 7: Cleanup Apply and Audit Log

**Rationale:** The final stage of the safety pipeline. Trash behavior on external volumes must be tested before this ships. The `--force` permanent delete requires double confirmation (`--force` AND `--yes`).
**Delivers:** `freespace clean apply` with trash-first deletion, `--force` guard, protected-path enforcement, cleanup audit log at `~/.local/state/freespace/cleanup.log`
**Avoids:** Pitfall 5 (Trash behavior on non-home-volume files) — test with mounted disk image before shipping

### Phase 8: Polish, Doctor, and Release Preparation

**Rationale:** `freespace doctor` requires all other commands to exist (it probes their preconditions). Release tooling (cargo-dist, cargo-deny) is additive and does not block earlier phases.
**Delivers:** `freespace doctor` with TCC probe and protected-path audit; cargo-dist release binaries; README with Full Disk Access instructions; shell completion generation
**Addresses:** Pitfall 4 (TCC) diagnosis path; operator confidence for users whose scans return incomplete results

### Phase Ordering Rationale

- The module build order from ARCHITECTURE.md (`config → platform → fs_scan → classify → analyze → output → cleanup → commands`) maps directly to the phase sequence. No shortcuts are viable without creating correctness gaps.
- All scan-layer pitfalls must be resolved in Phase 3 because downstream phases inherit whatever FileEntry.size means — changing from logical to physical bytes after classification is built requires recalculating all aggregates.
- Cleanup Apply (Phase 7) is deliberately last because it is the only phase with irreversible side effects. The research is explicit: "Never implement cleanup before scan and classification are reliable."
- Phase 4 (classification) precedes Phase 6 (cleanup preview) because every CleanupCandidate carries a safety field that comes from classify::safety(); there is no shortcut.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 4 (Classification Engine):** The macOS known-path registry requires broad research across macOS versions to enumerate all standard directories; new directories appear with each major macOS release. A research-phase pass to compile the complete known-path table before implementation reduces the risk of shipping an incomplete classifier.
- **Phase 7 (Cleanup Apply):** The trash crate's behavior on network volumes (SMB, AFP) is documented as limited. If external volume support is in scope, a research pass on trash-rs limitations and fallback strategies is warranted before implementation.

Phases with standard patterns (skip research-phase):
- **Phase 1 (Foundation):** clap 4 derive pattern, config via serde+toml, protected-path prefix matching — all well-documented.
- **Phase 2 (Volume Summary):** sysinfo Disks API is well-documented; statvfs usage via nix is standard.
- **Phase 3 (Scan Engine):** walkdir streaming pattern, inode deduplication with MetadataExt, st_blocks physical size — all established patterns with code examples in PITFALLS.md.
- **Phase 5 (Analysis Layer):** Pure fold + BinaryHeap pattern; no platform-specific behavior.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified against crates.io API on 2026-03-28; no unproven dependencies |
| Features | HIGH | Based on PRD + competitor matrix + macOS ecosystem research; feature dependency graph is well-reasoned |
| Architecture | HIGH | Pipeline structure matches reference implementations (dua-cli); module boundaries match Rust CLI best practices |
| Pitfalls | HIGH (macOS/TCC/SIP); MEDIUM (trash-rs external volume specifics) | TCC/SIP from official Apple sources; APFS clone behavior from documented community research; trash-rs external volume behavior from GitHub issues |

**Overall confidence:** HIGH

### Gaps to Address

- **APFS purgeable space API:** The exact `getattrlist`/`ATTR_VOL_SPACEUSED` call needed to surface purgeable space separately from available space has no stable high-level Rust wrapper. Will need manual nix/libc integration in `platform::macos`; test against a real APFS volume with local Time Machine snapshots.
- **macOS known-path registry completeness:** The registry must cover all directories a developer encounters. The research identifies key paths (`~/.ollama`, `~/Library/Developer/Xcode/DerivedData`, `~/.docker`, etc.) but macOS 26 and future releases will add new paths. Plan for the registry to be config-extensible in v1 (supplemented by `~/.config/freespace/config.toml` entries) so users can add paths without waiting for a code release.
- **trash-rs on network volumes:** The crate returns an error for SMB/AFP paths. The correct behavior (warn and skip, or warn and offer `--force` permanent delete) needs a design decision before Phase 7.

## Sources

### Primary (HIGH confidence)
- crates.io API — all stack version numbers verified directly, 2026-03-28
- Apple Developer Documentation — TCC, SIP, System Integrity Protection behavior
- sysinfo crate docs (docs.rs) — Disks API, statvfs blocking note
- walkdir docs (docs.rs) — WalkDir struct, follow_links, loop detection

### Secondary (MEDIUM confidence)
- Byron/trash-rs GitHub — macOS Trash implementation, external volume limitations (issue #8)
- The Eclectic Light Company — APFS clone sharing, snapshot purgeable space behavior (2022–2024)
- BurntSushi/walkdir GitHub — sequential walk design rationale
- Kevin K. (kbknapp.dev) — CLI structure in Rust; command handler patterns
- dua-cli GitHub (Byron) — reference implementation for streaming disk aggregation

### Tertiary (MEDIUM confidence)
- Rust CLI Patterns 2026-02 — clap derive pattern confirmation
- Rust Error Handling 2025 Guide — thiserror 2.0 / anyhow pattern for CLIs
- macOS developer disk space blog posts — cache directory locations, developer pain points

---
*Research completed: 2026-03-28*
*Ready for roadmap: yes*
