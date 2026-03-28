# Feature Research

**Domain:** Disk inspection and cleanup CLI utility (macOS, Rust)
**Researched:** 2026-03-28
**Confidence:** HIGH (PRD + competitor analysis + macOS ecosystem research)

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist in any disk inspection tool. Missing these = tool feels broken or incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Volume summary (total / used / free) | Every disk tool since `df` shows this; first thing users want | LOW | `freespace summary` — mirrors `duf`'s primary view |
| Recursive directory size scan | Core of `du`, `dust`, `ncdu`, `diskus` — the universal entry point | MEDIUM | `freespace scan <path>` — streaming traversal, no full tree in memory |
| Largest files and directories list | Users open disk tools specifically to find big files; ncdu, dust both lead with this | MEDIUM | `freespace largest <path>` — top-N configurable |
| File count and directory count | Context for interpreting sizes; all serious tools report this | LOW | Output alongside scan totals |
| Human-readable sizes (KB/MB/GB) | Non-negotiable UX baseline; raw bytes are unusable | LOW | comfy-table handles this |
| Graceful permission error handling | macOS restricts many paths; crashes on EPERM destroy trust | MEDIUM | Continue scan, log errors to stderr, never panic |
| Broken symlink and mid-scan deletion tolerance | Common in active systems; ncdu and dust both handle silently | MEDIUM | walkdir handles most cases; explicit logic needed for mid-scan deletes |
| Exclusion support (paths/patterns) | Users always have paths to skip (external drives, network mounts) | LOW | `~/.config/freespace/config.toml` exclude list |
| macOS-specific path awareness | `/System`, `~/Library`, `/private/var/folders` are macOS-specific; Linux tools miss this | MEDIUM | `platform::macos` module with known-path map |
| Progress indicator during scan | Large directories (100k+ files) take time; silence feels like a hang | LOW | Simple spinner or file count to stderr; suppressed with `--json` |

### Differentiators (Competitive Advantage)

Features that set Freespace apart from ncdu, dust, duf, and diskus. These are where the tool competes.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| 14-category classification system | No competitor classifies by semantic category (video/audio/images/documents/archives/apps/developer/caches/mail/containers/cloud-sync/hidden/system-related/unknown); macOS Storage panel does this but is not scriptable or inspectable | HIGH | `freespace categories <path>` — path-first, then extension fallback; `classify` module |
| Safety classification on cleanup candidates (safe/caution/dangerous/blocked) | ncdu and dua-cli delete directly with a keypress; no safety tier system exists anywhere. Freespace attaches a safety class to every cleanup target | HIGH | Four tiers: `safe` (caches, tmp), `caution` (logs, build artifacts), `dangerous` (user data), `blocked` (system paths) |
| Inspect → Classify → Preview → Clean enforced pipeline | All other tools let you delete at any point. Freespace enforces the pipeline as a hard constraint — cleanup cannot run before scan+classification are reliable | HIGH | Architectural guarantee, not a UI convention. Prevents accidental deletion by design |
| Dedicated cache inspection command | `freespace caches` surfaces all cache directories with reclaimable size and safety class. No competitor separates this from general scan. Critical for macOS where `~/Library/Caches` can hold 50GB+ | MEDIUM | Known macOS cache paths + extension scan; maps directly to developer pain points |
| Hidden file and dotfile audit | `freespace hidden <path>` — developers accumulate large hidden directories (`.ollama`, `.docker`, `.npm`, `.gradle`). No competitor surfaces this as a first-class command | MEDIUM | Filter entries where `is_hidden = true`; show total hidden size |
| Trash-first deletion with blocked paths | Most tools offer direct `rm`. Freespace defaults to macOS Trash via `trash` crate; permanent delete requires `--force`. Protected paths (`/System`, `/usr`, `/bin`, `/sbin`, `/private`) are immutable — cannot be deleted even with `--force` | MEDIUM | `trash` crate; blocked path list hardcoded + config-extensible |
| Cleanup preview as first-class command | `freespace clean preview` shows exactly what will be affected, how much space is reclaimed, and the safety class of each item — before a single byte is deleted. ncdu/dua have no preview concept | MEDIUM | `cleanup::preview` module; output mirrors `clean apply` output exactly |
| Full JSON output on all major commands | ncdu exports JSON for import only. dust has `-j`. Freespace makes `--json` canonical on every command: summary, scan, categories, caches, hidden, clean preview — enabling pipelines with jq, scripts, and monitoring systems | MEDIUM | Clean JSON on stdout; all logs/errors to stderr; enforced contract |
| Cleanup audit log | Every deletion action logged to `~/.local/state/freespace/cleanup.log`. No competitor provides this. Critical for trust: users can verify what was deleted and when | LOW | Append-only log with timestamp, path, size, action (trash/delete) |
| macOS developer-specific known paths | Maps `~/.ollama`, `~/Library/Developer/Xcode/DerivedData`, `~/Library/Caches/com.apple.*`, `~/.docker`, `node_modules` etc. to the correct category automatically. Linux-generic tools miss all of this | MEDIUM | `platform::macos` known-path registry; highest classification priority |
| `freespace doctor` diagnostic command | Self-diagnostic: checks config validity, permission access, protected path list integrity, log directory existence. Surfaces setup problems before they matter | LOW | Developer-friendly onboarding and debugging |

### Anti-Features (Commonly Requested, Often Problematic)

Features that seem like natural extensions but should be deliberately excluded.

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Interactive TUI (ncdu-style navigation) | ncdu is the gold standard; users expect interactive browsing | Adds significant complexity (ratatui/crossterm), scope-creeps MVP, conflicts with JSON-first scripting model, and interactive deletion bypasses the Inspect→Classify→Preview→Clean pipeline | Compose `scan` + `largest` + `categories` commands; pipe to `less` or `fzf` for interactive exploration |
| Automatic cleanup without preview | "One-command clean my machine" is appealing for speed | Violates the safety pipeline; users cannot reason about what was deleted; one wrong path classification causes data loss | `freespace clean preview` followed by explicit `freespace clean apply` — two steps is the minimum safe path |
| Deep Photos/Mail internals inspection | Users want to reclaim space from bloated Photos libraries | Photos and Mail use opaque internal databases; parsing them incorrectly corrupts libraries; Apple actively discourages third-party access | Classify `~/Pictures/Photos Library.photoslibrary` and `~/Library/Mail` as categories with size totals; surface path but do not inspect internals |
| Cross-platform support (Linux/Windows) | Wider user base | Removes the macOS-specific value (known path registry, Trash integration, macOS category semantics). Platform abstraction adds complexity that dilutes focus | `platform::macos` module isolates OS behavior; design for future expansion but do not implement now |
| AI-driven cleanup suggestions | Sounds smart; some users want "tell me what to delete" | Advisory AI requires model calls (latency, cost, privacy risk), hallucinations can suggest wrong deletions, opt-in model is complex to implement safely in MVP | Deterministic safety classification system delivers the same value without model risk; defer AI to v2, opt-in only, advisory only, never auto-delete |
| Real-time watch mode (auto-rescan on change) | Useful for monitoring growing directories | `FSEvents` integration adds complexity; streaming re-aggregation on changes is non-trivial; background processes create safety and resource questions | Users can re-run `freespace scan <path>` on demand; add `--watch` flag post-MVP if validated |
| Duplicate file detection | Natural disk cleanup feature | Requires content hashing (SHA-256 over all files), high I/O cost on large directories, complex dedup UI, out of scope for inspection-first tool | Not in scope for v1; could be a standalone `freespace dupes <path>` command in v2 |
| Application uninstaller | GUI cleaners like CleanMyMac include this | Requires tracking all app residuals across 20+ macOS locations; extremely high complexity; high risk of removing valid app data; outside the inspection-to-cleanup scope | `freespace categories <path>` surfaces `applications` category; users can investigate and delete manually with `clean apply` |
| Scheduled/automatic cleanup | Set-and-forget maintenance | Automatic deletion without human review violates the core safety philosophy; launchd integration adds platform complexity | Document `freespace clean preview` + `freespace clean apply` as a composable pattern for user-defined automation scripts |

---

## Feature Dependencies

```
[Volume Summary]
    (standalone — no dependencies)

[Directory Scan]
    └──required by──> [Largest Files/Dirs]
    └──required by──> [Category Classification]
                          └──required by──> [Hidden Audit]
                          └──required by──> [Cache Inspection]
                          └──required by──> [Cleanup Preview]
                                                └──required by──> [Cleanup Apply]

[Config System]
    └──enhances──> [Directory Scan] (exclusion paths)
    └──enhances──> [Cleanup Apply] (safe_categories list)

[Safety Classification]
    └──required by──> [Cleanup Preview]
    └──required by──> [Cleanup Apply]
    └──enhances──> [Cache Inspection] (per-cache safety tier)

[JSON Output]
    └──enhances──> all commands (--json flag)

[Cleanup Audit Log]
    └──required by──> [Cleanup Apply]

[Platform::macOS Known Paths]
    └──enhances──> [Category Classification] (path-first priority)
    └──enhances──> [Cache Inspection] (macOS-specific cache dirs)
```

### Dependency Notes

- **Cleanup Apply requires Cleanup Preview:** The pipeline is enforced architecturally — `clean apply` should not be implemented until `clean preview` is reliable and tested. This is a safety-critical constraint, not a UI suggestion.
- **Category Classification requires Directory Scan:** Classification operates on scan results (FileEntry structs). The classifier cannot run without a populated scan.
- **Safety Classification required by Cleanup Preview:** Every `CleanupCandidate` carries a `safety` field. Preview cannot render without safety tiers assigned.
- **Config System enhances Scan and Cleanup:** Exclusion paths and safe-category overrides are read from config; the tool works without config (defaults apply) but config unlocks user customization.
- **Platform::macOS Known Paths enhances Classification:** Path-first rules are the highest-priority classification signal. Without the macOS path registry, `~/Library/Caches/com.apple.Safari` would fall through to extension-based classification, yielding `unknown`.
- **JSON Output is a cross-cutting concern:** `--json` must be implemented consistently across all commands; inconsistent JSON shape breaks automation pipelines.

---

## MVP Definition

### Launch With (v1)

Minimum set that validates the core value proposition: "zero to safe cleanup in one session."

- [ ] `freespace summary` — shows mounted volumes with total/used/available; validates volume inspection
- [ ] `freespace scan <path>` — total size, file count, dir count, streaming; validates core traversal engine
- [ ] `freespace largest <path>` — top-N files and directories; the single most-used feature in every disk tool
- [ ] `freespace categories <path>` — 14-category breakdown; primary differentiator; validates classification engine
- [ ] `freespace hidden <path>` — dotfiles and hidden dirs with size totals; developer-specific value
- [ ] `freespace caches` — cache dirs with reclaimable size and safety class; highest immediate value for macOS users
- [ ] `freespace clean preview` — what would be deleted, how much space, safety class; enforces pipeline
- [ ] `freespace clean apply` — trash-first deletion, `--force` for permanent, blocked paths enforced
- [ ] `--json` on all major commands — enables scripting from day one
- [ ] `~/.config/freespace/config.toml` — exclusion paths and safe-category overrides
- [ ] Cleanup audit log at `~/.local/state/freespace/cleanup.log`
- [ ] Graceful error handling: permission denied, broken symlinks, mid-scan deletes

### Add After Validation (v1.x)

- [ ] `freespace doctor` — add once core commands are stable; helps with user onboarding issues
- [ ] `--watch` mode for scan — add if users request continuous monitoring; validate demand first
- [ ] Configurable top-N for largest files — simple flag addition once core UX is understood

### Future Consideration (v2+)

- [ ] Duplicate file detection (`freespace dupes <path>`) — high I/O cost; validate demand before investing
- [ ] Interactive TUI mode — only if JSON-first workflow proves insufficient for enough users; adds major complexity
- [ ] AI-assisted classification for unknown files — opt-in, advisory only, never auto-delete; requires privacy review
- [ ] Linux platform support — `platform::linux` module; only after macOS is stable and validated
- [ ] `freespace uninstall <app>` — app residual cleanup; extremely high complexity; post-product-fit only

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Volume summary | HIGH | LOW | P1 |
| Directory scan (streaming) | HIGH | MEDIUM | P1 |
| Largest files/dirs | HIGH | MEDIUM | P1 |
| 14-category classification | HIGH | HIGH | P1 |
| Cache inspection with safety class | HIGH | MEDIUM | P1 |
| Cleanup preview | HIGH | MEDIUM | P1 |
| Cleanup apply (trash-first) | HIGH | MEDIUM | P1 |
| Safety classification system (4 tiers) | HIGH | HIGH | P1 |
| JSON output on all commands | HIGH | LOW | P1 |
| Hidden file audit | MEDIUM | LOW | P1 |
| Config system (TOML) | MEDIUM | LOW | P1 |
| Cleanup audit log | MEDIUM | LOW | P1 |
| Graceful error handling | HIGH | MEDIUM | P1 |
| macOS known-path registry | HIGH | MEDIUM | P1 |
| `freespace doctor` | MEDIUM | LOW | P2 |
| `--watch` mode | LOW | MEDIUM | P3 |
| Duplicate detection | MEDIUM | HIGH | P3 |
| Interactive TUI | MEDIUM | HIGH | P3 |
| AI-assisted classification | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

---

## Competitor Feature Analysis

| Feature | ncdu | dust | duf | diskus | Freespace |
|---------|------|------|-----|--------|-----------|
| Volume summary | No | No | Yes (primary feature) | No | Yes (`summary`) |
| Recursive scan | Yes (interactive) | Yes (tree + bars) | No | Yes (total only) | Yes (streaming) |
| Largest files/dirs | Yes (sorted list) | Yes (visual bars) | No | No | Yes (top-N) |
| Category classification | No | No | No | No | Yes (14 categories) |
| Safety tiers on cleanup | No | No | No | No | Yes (safe/caution/dangerous/blocked) |
| Cleanup preview | No | No | No | No | Yes (first-class command) |
| Delete files | Yes (interactive, permanent) | No | No | No | Yes (trash-first, `--force` for permanent) |
| Protected path blocking | No | No | No | No | Yes (hardcoded + config) |
| Cache inspection | No | No | No | No | Yes (`caches` command) |
| Hidden file audit | No | No | No | No | Yes (`hidden` command) |
| JSON output | Import/export only | Yes (`-j`) | Yes | No | Yes (all commands, `--json`) |
| Config file | No | Yes | No | No | Yes (TOML) |
| Cleanup audit log | No | No | No | No | Yes |
| macOS-specific paths | Partial | No | Partial | No | Yes (dedicated module) |
| Interactive TUI | Yes (ncurses) | No | No | No | No (deliberate) |
| Cross-platform | Yes | Yes | Yes | Yes | No (macOS-only, deliberate) |

---

## Sources

- ncdu manual and features: https://dev.yorhel.nl/ncdu/man
- dust GitHub repository: https://github.com/bootandy/dust
- duf GitHub repository: https://github.com/muesli/duf
- diskus GitHub repository: https://github.com/sharkdp/diskus
- dua-cli GitHub repository: https://github.com/Byron/dua-cli
- macOS developer disk space analysis: https://dissectmac.com/blog/clean-xcode-derived-data
- macOS Storage panel expectations: https://support.apple.com/en-us/102624
- macOS developer cache cleanup patterns: https://medium.com/@ojhakrishnabahadur010/how-to-free-up-space-on-macos-as-a-developer-20fa9bd2e3a9
- ClearDisk developer cache cleaner: https://bysiber.github.io/cleardisk/
- Freespace PRD: `/Users/giorgos/Workspace/AI-Safe/GSD-Projects/Freespace/Freespace_Full-PRD-Spec.md`
- Freespace PROJECT.md: `/Users/giorgos/Workspace/AI-Safe/GSD-Projects/Freespace/.planning/PROJECT.md`

---
*Feature research for: disk inspection and cleanup CLI utility (macOS, Rust)*
*Researched: 2026-03-28*
