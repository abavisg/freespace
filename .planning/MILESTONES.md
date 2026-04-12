# Milestones

## v1.0 MVP (Shipped: 2026-04-12)

**Phases completed:** 8 phases, 11 plans, 18 tasks

**Key accomplishments:**

- clap 4.6 CLI skeleton with 9 subcommands, cfg-gated macOS platform module, config/output stubs, and all Phase 1-8 dependencies declared — cargo build and cargo test both pass clean
- Full config loader, protected-path canonicalization, and global --json wiring with 23 passing unit tests covering all 6 FOUND requirements
- `freespace summary` enumerates real disk volumes via sysinfo, renders a comfy-table with human-readable sizes, and emits a clean JSON array via `--json` — 32 tests green including 6 new integration tests
- 1. [Rule 1 - Bug] exclude field is Vec<String> not Vec<PathBuf>
- 1. [Rule 1 - Bug] Fixed hidden classification for files inside hidden directories
- Hidden command lists dotfiles at top level using read_dir to avoid double-counting; caches command discovers 7 macOS cache dirs, classifies safety, and computes reclaimable total from Safe entries
- BinaryHeap top-N file tracking and HashMap directory rollup wired into scan_path, backed by freespace largest command with table and JSON output
- classify/mod.rs
- Status:
- freespace doctor fully implemented: 4-check TCC/FDA diagnostic command with comfy_table human output, JSON mode, actionable remediation messages, and exit-code semantics (0=pass/warn, 1=fail)

---
