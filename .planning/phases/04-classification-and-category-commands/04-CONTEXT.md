# Phase 4: Classification and Category Commands - Context

**Gathered:** 2026-03-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 4 delivers the classification engine and three commands that depend on it: `freespace categories <path>`, `freespace hidden <path>`, and `freespace caches`. After this phase: disk usage is grouped into all 14 semantic categories, hidden files/dirs are listed with sizes, and cache directories are discovered with safety classifications. This is the primary differentiator — no competitor tool has semantic classification.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All implementation choices are at Claude's discretion. Key constraints from the PRD and research:

**Classification priority order (MUST be followed):**
1. Path rules (highest priority)
2. Known macOS dirs (~/Library/Caches → caches, ~/Library/Mail → mail, ~/.ollama → developer, etc.)
3. Extension mapping (video/audio/images/documents/archives/applications)
4. Fallback → unknown

**14 categories (ALL must be present):**
video, audio, images, documents, archives, applications, developer, caches, mail, containers, cloud-sync, hidden, system-related, unknown

**Safety classification for caches (safe/caution/dangerous/blocked):**
- safe: standard user cache dirs (~/Library/Caches/*)
- caution: app-specific caches that may break functionality if deleted
- dangerous: system-adjacent caches
- blocked: protected paths

**Architecture:**
- New module: `src/classify/mod.rs` — pure functions, no I/O, easily testable
- `classify::classify_path(path: &Path) -> Category` — main classification entry point
- `classify::safety_class(path: &Path) -> SafetyClass` — for caches command
- `commands/categories.rs`, `commands/hidden.rs`, `commands/caches.rs` — thin dispatch layers, stubs exist

**Key macOS known paths to classify:**
- ~/Library/Caches → caches
- ~/Library/Mail → mail
- ~/Library/Containers → containers
- ~/.Trash → unknown (never clean)
- ~/.ollama, ~/Library/Developer, DerivedData → developer
- ~/Library/CloudStorage, ~/.dropbox, ~/Library/Mobile Documents → cloud-sync
- /System, /usr, /bin → system-related
- Dotfiles/hidden → hidden

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/fs_scan/mod.rs` — `scan_path()` streaming engine, use for traversal
- `src/output/mod.rs` — `write_json()` and table output
- `src/platform/macos.rs` — `protected_paths()` for blocked classification
- `src/commands/categories.rs`, `hidden.rs`, `caches.rs` — stubs exist
- All deps already in Cargo.toml

### Established Patterns
- anyhow::Result in command handlers
- #[derive(Serialize)] on data structs for JSON
- comfy-table for table output
- tracing for stderr logging

### Integration Points
- `classify/mod.rs` → called by categories.rs, hidden.rs, caches.rs
- categories/hidden/caches commands → `fs_scan::scan_path()` for traversal
- categories/hidden/caches commands → `output::write_json()` for JSON

</code_context>

<specifics>
## Specific Ideas

Extension mappings to implement:
- video: mp4, mov, avi, mkv, m4v, wmv, flv, webm
- audio: mp3, aac, flac, wav, m4a, ogg, opus
- images: jpg, jpeg, png, gif, bmp, tiff, webp, heic, raw
- documents: pdf, doc, docx, xls, xlsx, ppt, pptx, txt, md, pages, numbers, keynote
- archives: zip, tar, gz, bz2, xz, 7z, rar, dmg, pkg
- applications: app, ipa, apk

</specifics>

<deferred>
## Deferred Ideas

None — all classification features are in scope for this phase.

</deferred>
