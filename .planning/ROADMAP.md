# Roadmap: Freespace

## Overview

Freespace is built in strict safety order: foundation and scaffolding first, then scan reliability, then classification, then analysis, then cleanup preview, and finally cleanup apply — the only phase with irreversible side effects. This ordering is architectural, not arbitrary: every phase depends on the correctness of the phase before it. Doctor and polish ship last because they require all other commands to exist.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Foundation** - CLI skeleton, protected-path scaffolding, config system, error handling, --json wiring (completed 2026-03-28)
- [x] **Phase 2: Volume Summary** - `freespace summary` showing mounted volumes with total/used/available space (completed 2026-03-29)
- [ ] **Phase 3: Core Scan Engine** - Streaming scan with hardlink dedup, physical-size accounting, TCC tolerance
- [ ] **Phase 4: Classification and Category Commands** - 14-category classifier, categories/hidden/caches subcommands
- [ ] **Phase 5: Analysis Layer and Largest Files** - `freespace largest` powered by fold+BinaryHeap aggregation
- [ ] **Phase 6: Cleanup Preview** - Read-only preview gate; no deletions until preview is verified
- [ ] **Phase 7: Cleanup Apply** - Trash-first deletion, --force guard, protected-path enforcement, audit log
- [ ] **Phase 8: Doctor and Polish** - Self-diagnostics, TCC probe, release tooling, shell completions

## Phase Details

### Phase 1: Foundation
**Goal**: Users can invoke any Freespace subcommand and get a meaningful response; the safety scaffolding (protected paths, config, error routing) is in place before any real logic is built
**Depends on**: Nothing (first phase)
**Requirements**: FOUND-01, FOUND-02, FOUND-03, FOUND-04, FOUND-05, FOUND-06
**Success Criteria** (what must be TRUE):
  1. Running `freespace --help` lists all subcommands (summary, scan, largest, categories, hidden, caches, clean preview, clean apply, config, doctor)
  2. Running any subcommand with `--json` produces clean JSON on stdout and no JSON on stderr
  3. Protected paths (/System, /usr, /bin, /sbin, /private) are resolved via canonicalize() and stored as constants at startup
  4. `~/.config/Freespace/config.toml` is read on startup; missing file is handled gracefully without crash
  5. All error output goes to stderr; stdout is reserved for structured output only
**Plans**: 2 plans

Plans:
- [ ] 01-01-PLAN.md — Create project scaffold: cargo new, Cargo.toml with all deps, all source file stubs (compile-clean)
- [ ] 01-02-PLAN.md — Implement real logic: config loader, platform::macos protected paths, cli --json global flag, output module, comprehensive test suite

### Phase 2: Volume Summary
**Goal**: Users can see all mounted volumes with their disk usage at a glance
**Depends on**: Phase 1
**Requirements**: SUMM-01, SUMM-02
**Success Criteria** (what must be TRUE):
  1. `freespace summary` prints a table listing every mounted volume with mount point, total bytes, used bytes, and available bytes in human-readable form
  2. `freespace summary --json` produces clean JSON on stdout with the same fields, logs on stderr only
**Plans**: 1 plan

Plans:
- [ ] 02-01-PLAN.md — Implement VolumeInfo + list_volumes() in platform::macos, summary command dispatch, table and JSON output, integration tests

### Phase 3: Core Scan Engine
**Goal**: Users can scan any path and get accurate, deduplicated, physically-sized results — and the scan never crashes on permission errors or broken symlinks
**Depends on**: Phase 2
**Requirements**: SCAN-01, SCAN-02, SCAN-03, SCAN-04, SCAN-05
**Success Criteria** (what must be TRUE):
  1. `freespace scan <path>` reports total size, file count, directory count — and the size reflects physical allocation (st_blocks * 512), not logical length
  2. Hardlinked files are counted only once; scanning a directory with hardlinks does not inflate the reported total
  3. Scanning a path containing TCC-protected subdirectories completes without crashing; skipped paths are counted and shown in the summary
  4. Scanning a path with broken symlinks or files deleted mid-scan completes without crashing
  5. The scanner operates as a streaming iterator and does not load full directory trees into memory
**Plans**: TBD

### Phase 4: Classification and Category Commands
**Goal**: Users can see disk usage broken down by semantic category, inspect hidden files, and view safety-classified cache directories — all powered by a correct macOS-aware classifier
**Depends on**: Phase 3
**Requirements**: CAT-01, CAT-02, CAT-03, HIDD-01, HIDD-02, CACH-01, CACH-02, CACH-03
**Success Criteria** (what must be TRUE):
  1. `freespace categories <path>` groups all files into all 14 categories (video, audio, images, documents, archives, applications, developer, caches, mail, containers, cloud-sync, hidden, system-related, unknown) with total bytes and file count per category
  2. Classification uses path rules first, then macOS known dirs (~/Library/Caches, ~/.ollama, etc.), then extension, then unknown — macOS-specific paths are never misclassified as unknown
  3. `freespace hidden <path>` lists dotfiles and hidden directories with individual sizes and a total hidden size for the path
  4. `freespace caches` discovers cache directories across standard macOS locations with path, size, and safety classification (safe/caution/dangerous/blocked) per entry
  5. Reclaimable total across safe-classified caches is shown in `freespace caches` output
**Plans**: TBD

### Phase 5: Analysis Layer and Largest Files
**Goal**: Users can identify the largest files and directories at any path using an efficient, memory-bounded aggregation engine
**Depends on**: Phase 4
**Requirements**: SCAN-06
**Success Criteria** (what must be TRUE):
  1. `freespace largest <path>` reports the top-N largest files and directories in a human-readable table
  2. `freespace largest <path> --json` produces clean JSON output with the same data
  3. The aggregation uses a bounded BinaryHeap (not a full sort) so memory usage stays bounded regardless of directory size
**Plans**: TBD

### Phase 6: Cleanup Preview
**Goal**: Users can see exactly what a cleanup would affect — including safety classification and total reclaimable space — before any file is touched
**Depends on**: Phase 5
**Requirements**: PREV-01, PREV-02, PREV-03
**Success Criteria** (what must be TRUE):
  1. `freespace clean preview` shows every file/directory that would be affected with safety classification and individual size
  2. Total reclaimable space is shown across all preview candidates
  3. Running `freespace clean preview` makes no changes to disk — no files are moved, modified, or deleted
  4. `freespace clean preview --json` produces clean JSON output of preview candidates on stdout
**Plans**: TBD

### Phase 7: Cleanup Apply
**Goal**: Users can safely reclaim disk space — with Trash as the default, permanent deletion behind --force, and protected paths immutably blocked under all circumstances
**Depends on**: Phase 6
**Requirements**: APPLY-01, APPLY-02, APPLY-03, APPLY-04, APPLY-05
**Success Criteria** (what must be TRUE):
  1. `freespace clean apply` moves targeted files to macOS Trash; files are recoverable from Finder Trash afterward
  2. Permanent deletion only proceeds when `--force` is explicitly provided; without it, the command refuses to permanently delete
  3. Attempting to delete any file under /System, /usr, /bin, /sbin, or /private (or their symlink aliases) is blocked and logged — the deletion never executes
  4. Every cleanup action is appended to `~/.local/state/Freespace/cleanup.log` with timestamp, path, size, and action type
  5. `freespace clean apply` without a prior scan and classification pass fails with an informative error — it cannot be invoked as a first command
**Plans**: TBD

### Phase 8: Doctor and Polish
**Goal**: Users can diagnose permission and configuration issues with a self-check command, and the tool is ready for distribution
**Depends on**: Phase 7
**Requirements**: DIAG-01, DIAG-02
**Success Criteria** (what must be TRUE):
  1. `freespace doctor` reports Full Disk Access (TCC) status, protected-path verification results, and config file validity in a single pass
  2. Each issue detected by doctor includes an actionable remediation message — not just "error detected"
  3. `freespace doctor` exits with a non-zero code when any check fails, enabling scripted health checks
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 1/2 | Complete    | 2026-03-28 |
| 2. Volume Summary | 1/1 | Complete    | 2026-03-29 |
| 3. Core Scan Engine | 0/TBD | Not started | - |
| 4. Classification and Category Commands | 0/TBD | Not started | - |
| 5. Analysis Layer and Largest Files | 0/TBD | Not started | - |
| 6. Cleanup Preview | 0/TBD | Not started | - |
| 7. Cleanup Apply | 0/TBD | Not started | - |
| 8. Doctor and Polish | 0/TBD | Not started | - |
