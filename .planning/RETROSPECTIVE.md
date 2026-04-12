# Retrospective: Freespace

## Milestone: v1.0 — MVP

**Shipped:** 2026-04-12
**Phases:** 8 | **Plans:** 11 | **Tasks:** ~18

### What Was Built

- Foundation: clap CLI skeleton, platform module, config system, protected paths, --json global flag
- Volume summary: sysinfo-backed mounted volume enumeration with human/JSON output
- Core scan engine: streaming walkdir, hardlink dedup via (dev,ino), physical-size via st_blocks, TCC-tolerant
- Classification: 14-category macOS-aware classifier (path-first → known-dirs → extension → unknown)
- Hidden + caches commands: dotfile listing, cache dir discovery with safety classification
- Largest files: BinaryHeap top-N aggregation, memory-bounded, O(N log k)
- Cleanup preview: read-only gate with safety classification and reclaimable total
- Cleanup apply: Trash-first deletion, --force guard, protected-path enforcement, audit log
- Doctor: TCC/FDA probe, protected-path check, config/log checks, actionable remediation per check
- Shell completions: zsh/bash/fish via clap_complete

### What Worked

- **TDD wave structure (Plan 01 RED → Plan 02 GREEN)** — writing failing tests first in Phase 8 gave Plan 02 a crystal-clear acceptance contract, zero ambiguity about what "done" meant
- **Strict phase ordering** — building scan before classification before cleanup prevented entire classes of bugs; no rework needed due to ordering mistakes
- **clap derive API** — negligible boilerplate, all flag/subcommand wiring was correct first time
- **anyhow + thiserror** — consistent error propagation; stderr-only logs worked without any special handling

### What Was Inefficient

- **Requirements checkbox tracking** — 10 requirements (HIDD, CACH, APPLY) were implemented but their checkboxes weren't ticked during execution; had to fix at milestone close. Could be automated post-phase.
- **Binary reinstall gap** — `freespace completions zsh` failed in UAT because the installed binary was stale; `cargo install --path .` was needed. Worth noting in project CLAUDE.md.
- **Summary one-liner quality** — several SUMMARY.md files had poor `one_liner` fields (bugs listed instead of accomplishments). The summary-extract tool picks these up verbatim.

### Patterns Established

- Session file pattern for Inspect→Classify→Preview→Clean enforcement: `~/.local/state/Freespace/freespace-preview.json` written by preview, consumed by apply
- cfg-gated platform module: all macOS-specific code behind `#[cfg(target_os = "macos")]` with non-macOS fallbacks
- comfy_table output style: `Table::new()` (no preset), `set_header`, `add_row` — consistent across all 8 commands

### Key Lessons

- Keep the installed binary in sync with the repo during UAT — add `cargo install --path freespace/` to the project CLAUDE.md
- Fix requirement checkboxes immediately after each phase completes — don't defer to milestone close
- Wave 0 RED test scaffold pattern (Plan 01) works exceptionally well for final implementation phases — highly recommended for any phase with a clear JSON contract

### Cost Observations

- Build completed in 15 calendar days (2026-03-28 → 2026-04-12)
- 79 total commits, 19 feature commits
- Model: claude-sonnet-4-6 throughout (executor + verifier + orchestrator)

---

## Cross-Milestone Trends

| Milestone | Phases | Plans | Days | Rework |
|-----------|--------|-------|------|--------|
| v1.0 MVP | 8 | 11 | 15 | Low |
