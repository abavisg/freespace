# Requirements: Freespace

**Defined:** 2026-03-28
**Core Value:** A power user can go from zero knowledge to safe, informed disk cleanup in a single session — with no surprises and no accidental deletions.

## v1 Requirements

### Foundation

- [x] **FOUND-01**: CLI skeleton with all subcommands routed via clap derive API (summary, scan, largest, categories, hidden, caches, clean preview, clean apply, config, doctor)
- [x] **FOUND-02**: Platform module (`platform::macos`) isolates all macOS-specific behavior behind `#[cfg(target_os = "macos")]`
- [x] **FOUND-03**: Protected-path constants (/System, /usr, /bin, /sbin, /private) resolved via `canonicalize()` to prevent symlink bypass
- [x] **FOUND-04**: Config system reads `~/.config/Freespace/config.toml` with `[scan] exclude` and `[cleanup] safe_categories` support
- [x] **FOUND-05**: Error handling uses thiserror for domain errors and anyhow in command handlers; all logs/errors go to stderr only
- [x] **FOUND-06**: `--json` flag wired globally — all commands support it; JSON output is clean stdout only

### Inspection — Volume Summary

- [ ] **SUMM-01**: `freespace summary` lists all mounted volumes with mount point, total bytes, used bytes, and available bytes
- [ ] **SUMM-02**: Summary output is human-readable table by default and clean JSON with `--json`

### Inspection — Scan

- [ ] **SCAN-01**: `freespace scan <path>` reports total size, file count, directory count, largest files (top-N), and largest directories (top-N)
- [ ] **SCAN-02**: Scanner uses streaming walkdir traversal — no loading full directory trees into memory
- [ ] **SCAN-03**: Scanner deduplicates hardlinks via `(dev, ino)` tracking to prevent double-counting
- [ ] **SCAN-04**: Scanner uses physical size (`st_blocks * 512`) for sparse files, not logical `metadata().len()`
- [ ] **SCAN-05**: Scanner handles permission errors, broken symlinks, and files deleted during scan without crashing — skipped paths are counted and surfaced
- [ ] **SCAN-06**: `freespace largest <path>` reports top-N largest files and directories at a path

### Inspection — Categories

- [ ] **CAT-01**: `freespace categories <path>` groups disk usage into all 14 categories: video, audio, images, documents, archives, applications, developer, caches, mail, containers, cloud-sync, hidden, system-related, unknown
- [ ] **CAT-02**: Classification priority order: path rules → known macOS dirs (~/Library/Caches, ~/Library/Mail, ~/.ollama, etc.) → extension mapping → unknown fallback
- [ ] **CAT-03**: Each category entry shows total bytes and file count

### Inspection — Hidden

- [ ] **HIDD-01**: `freespace hidden <path>` lists dotfiles and hidden directories with individual sizes
- [ ] **HIDD-02**: Hidden scan reports total hidden size for the scanned path

### Cleanup — Caches

- [ ] **CACH-01**: `freespace caches` discovers cache directories across standard macOS locations
- [ ] **CACH-02**: Each cache entry shows path, size, and safety classification (safe/caution/dangerous/blocked)
- [ ] **CACH-03**: Reclaimable total is shown across all safe-classified caches

### Cleanup — Preview

- [ ] **PREV-01**: `freespace clean preview` shows all files/directories that would be affected, total reclaimable space, and safety classification per item
- [ ] **PREV-02**: Preview is read-only — no files are modified or deleted during preview
- [ ] **PREV-03**: Preview output is human-readable table by default and clean JSON with `--json`

### Cleanup — Apply

- [ ] **APPLY-01**: `freespace clean apply` moves files to macOS Trash by default using the `trash` crate
- [ ] **APPLY-02**: Permanent deletion requires explicit `--force` flag
- [ ] **APPLY-03**: Protected paths (/System, /usr, /bin, /sbin, /private and their symlink aliases) cannot be deleted under any circumstances
- [ ] **APPLY-04**: All cleanup actions are logged to `~/.local/state/Freespace/cleanup.log` with timestamp, path, size, and action type
- [ ] **APPLY-05**: Cleanup apply cannot run without a prior scan and classification pass (enforces Inspect→Classify→Preview→Clean order)

### Diagnostics

- [ ] **DIAG-01**: `freespace doctor` runs self-diagnostics: TCC/Full Disk Access status, protected-path verification, config file validity
- [ ] **DIAG-02**: Doctor reports actionable remediation for each detected issue

## v2 Requirements

### Performance

- **PERF-01**: Parallel directory traversal via `jwalk` for large volumes (NVMe optimization)
- **PERF-02**: Progress indicator for long-running scans

### Output

- **OUT-01**: Interactive TUI mode (deliberately deferred — conflicts with JSON-first scripting philosophy)
- **OUT-02**: Export scan results to file

### Platform

- **PLAT-01**: Linux support (platform module already isolates this; expand later)
- **PLAT-02**: Windows support

### AI Features

- **AI-01**: Cleanup suggestions (opt-in only, advisory only, never auto-delete)
- **AI-02**: Edge-case classification assistance

## Out of Scope

| Feature | Reason |
|---------|--------|
| Exact replication of macOS Storage UI | Not the goal — terminal-native UX |
| Automatic cleanup without preview | Violates Inspect→Classify→Preview→Clean safety order |
| Deep system-level integrations (Photos, Mail internals) | Too risky for MVP |
| Interactive TUI (ncdu-style) | Conflicts with JSON-first scripting and pipeline enforcement |
| GUI or web interface | Terminal-first, scriptable |
| Cross-platform v1 | macOS-only; platform module isolates for future expansion |
| AI-driven features in v1 | Explicitly deferred; opt-in advisory only when added |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| FOUND-01 | Phase 1 | Complete |
| FOUND-02 | Phase 1 | Complete |
| FOUND-03 | Phase 1 | Complete |
| FOUND-04 | Phase 1 | Complete |
| FOUND-05 | Phase 1 | Complete |
| FOUND-06 | Phase 1 | Complete |
| SUMM-01 | Phase 2 | Pending |
| SUMM-02 | Phase 2 | Pending |
| SCAN-01 | Phase 3 | Pending |
| SCAN-02 | Phase 3 | Pending |
| SCAN-03 | Phase 3 | Pending |
| SCAN-04 | Phase 3 | Pending |
| SCAN-05 | Phase 3 | Pending |
| CAT-01 | Phase 4 | Pending |
| CAT-02 | Phase 4 | Pending |
| CAT-03 | Phase 4 | Pending |
| HIDD-01 | Phase 4 | Pending |
| HIDD-02 | Phase 4 | Pending |
| CACH-01 | Phase 4 | Pending |
| CACH-02 | Phase 4 | Pending |
| CACH-03 | Phase 4 | Pending |
| SCAN-06 | Phase 5 | Pending |
| PREV-01 | Phase 6 | Pending |
| PREV-02 | Phase 6 | Pending |
| PREV-03 | Phase 6 | Pending |
| APPLY-01 | Phase 7 | Pending |
| APPLY-02 | Phase 7 | Pending |
| APPLY-03 | Phase 7 | Pending |
| APPLY-04 | Phase 7 | Pending |
| APPLY-05 | Phase 7 | Pending |
| DIAG-01 | Phase 8 | Pending |
| DIAG-02 | Phase 8 | Pending |

**Coverage:**
- v1 requirements: 32 total
- Mapped to phases: 32
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-28*
*Last updated: 2026-03-28 after roadmap creation — SCAN-06 moved to Phase 5; CACH-01/02/03 moved to Phase 4*
