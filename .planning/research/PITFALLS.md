# Pitfalls Research

**Domain:** Rust CLI disk utility — scanning, classification, and safe cleanup on macOS
**Researched:** 2026-03-28
**Confidence:** HIGH (macOS filesystem behavior); MEDIUM (walkdir/trash-rs specifics from docs + community); HIGH (TCC/SIP from official Apple sources)

---

## Critical Pitfalls

### Pitfall 1: Hardlink Double-Counting Inflates Size Totals

**What goes wrong:**
When two or more directory entries share the same inode (hardlinks), naively summing `metadata().len()` counts those bytes once per directory entry. A 500 MB VM image with 5 hardlinks appears as 2.5 GB used. The scan total is wrong, and cleanup estimates become meaningless.

**Why it happens:**
`walkdir` emits one `DirEntry` per filesystem path, not per inode. The standard `metadata().len()` returns the logical file size, not a de-duplicated physical size. Nothing in the default traversal loop warns you that inode 12345 has already been counted.

**How to avoid:**
Track a `HashSet<(u64 device_id, u64 inode)>` during traversal. On each file entry, call `metadata().ino()` and `metadata().dev()` via `std::os::unix::fs::MetadataExt`. Only add bytes to the running total if the `(dev, ino)` pair has not been seen before. Use `(dev, ino)` not just `ino` because inode numbers are only unique per device.

```rust
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;

let mut seen_inodes: HashSet<(u64, u64)> = HashSet::new();
// inside traversal loop:
let meta = entry.metadata()?;
if meta.is_file() {
    let key = (meta.dev(), meta.ino());
    if seen_inodes.insert(key) {
        // first time seeing this inode — count it
        total += meta.len();
    }
}
```

**Warning signs:**
- Scan totals significantly exceed what `du -sh` reports for the same directory
- Any path known to contain Docker layer overlays, Git object stores, or Time Machine backup directories shows implausibly large sizes

**Phase to address:** Scan (fs_scan module, milestone 3) — must be correct before categories or cleanup can be trusted.

---

### Pitfall 2: Symlink Loops Hang or Crash the Traversal

**What goes wrong:**
With `follow_links(true)` on `walkdir::WalkDir`, a symlink pointing to an ancestor directory creates an infinite traversal loop. Even with loop detection enabled, misconfiguration (or assumptions about default behavior) can result in the process spinning until killed, or emitting a flood of errors that choke the output.

**Why it happens:**
`walkdir` does detect symlink loops when `follow_links(true)` is set — it emits a `WalkDir` error with `is_loop()` returning `true`. However, the loop error must be explicitly handled in the iterator. If the developer uses `.filter_map(|e| e.ok())` to skip errors, loop errors are silently dropped and traversal stalls at the looping entry. If errors propagate unchecked, the entire scan aborts at the first loop.

**How to avoid:**
- Keep `follow_links(false)` as the default (this is `walkdir`'s default). Report symlinks as symlinks, not as their targets.
- If following links is ever needed, handle loop errors explicitly: check `err.loop_ancestor()` and emit a warning to stderr, then continue.
- In tests, create a fixture directory with a symlink cycle and verify the scanner exits cleanly.

```rust
for entry in WalkDir::new(path).follow_links(false) {
    match entry {
        Ok(e) => process(e),
        Err(e) if e.loop_ancestor().is_some() => {
            eprintln!("warn: symlink loop at {:?}", e.path());
        }
        Err(e) => {
            eprintln!("warn: scan error at {:?}: {}", e.path(), e);
        }
    }
}
```

**Warning signs:**
- Scanner appears to hang on directories containing Docker volumes, cloud-sync folders (Dropbox, iCloud Drive), or user-created symlink structures
- CPU usage climbs without progress during a scan

**Phase to address:** Scan (fs_scan module, milestone 3) — symlink handling must be explicit from the first traversal implementation.

---

### Pitfall 3: APFS Clone / Snapshot Space Accounting Is Not What Users Expect

**What goes wrong:**
`du` and `metadata().len()` report the logical file size of APFS clones as if each clone occupies full space independently. Two clones of a 10 GB video file report as 20 GB used, even though they share underlying data blocks until one diverges. Similarly, APFS snapshots (Time Machine local snapshots, macOS update snapshots, SSV) hold onto blocks that look "available" to `statvfs` but cannot actually be written to until the snapshot is purged.

This produces two failure modes:
1. **Scan overcounts:** The tool reports 40 GB of video files; the user reclaims 20 GB; actual space recovered is close to zero because the clones still share blocks.
2. **Available space misreports:** `f_bavail * f_frsize` from `statvfs` may not account for purgeable snapshot space, so "X GB available" can be wrong by gigabytes.

**Why it happens:**
APFS clone detection requires the `APFS_IOC_GET_CLONE_INFO` ioctl or equivalent private API, which has no stable public interface. `du` has the same limitation. There is no POSIX-level way to determine whether two files share extents.

For available space: macOS adds a "purgeable" category to disk space that `statvfs.f_bavail` does not consistently reflect. The Finder shows available + purgeable; `df` and `statvfs` may show only available (without purgeable space that macOS would reclaim on demand).

**How to avoid:**
- Document the limitation explicitly in the tool's output. When reporting total size for a scanned path, add a note that APFS clones may cause size figures to be overstated.
- For volume available space (`freespace summary`), use `statvfs` but also query the macOS-specific `ATTR_VOL_SPACEUSED` or call `NSFileSystemFreeSize` via the `getattrlist` syscall in the `platform::macos` module to surface purgeable space separately.
- Do not promise "you will recover X GB" — say "up to X GB may be recovered."
- In `PITFALLS.md` for users: call out that large deltas between reported size and recovered space are expected on APFS volumes with clones.

**Warning signs:**
- Scan of `~/Library/Containers/com.docker.docker` or development directories shows impossibly large totals
- Available space reported by the tool differs substantially from Finder's "Available" figure
- After a cleanup the user reports little to no actual space was freed

**Phase to address:** Summary (milestone 2) for the available-space problem; Scan (milestone 3) for clone overcounting; Cleanup Preview (milestone 6) for the "recoverable bytes" estimate caveat.

---

### Pitfall 4: macOS TCC (Privacy) Silently Denies Access Without Crashing

**What goes wrong:**
On macOS Mojave and later, the TCC (Transparency, Consent, and Control) privacy layer denies access to sensitive directories — Desktop, Documents, Downloads, Mail, Messages, Photos, Safari data, and parts of `~/Library` — regardless of Unix file permissions and regardless of `sudo`. The denial returns `EPERM` (os error 1, "operation not permitted"), which looks identical to a genuine permissions error.

For a disk scanner, this means: the scan silently skips entire subtrees that users specifically want to analyse. The tool appears to work, reports a smaller total than reality, and the user cannot tell what was missed.

**Why it happens:**
Two independent access-control layers exist on macOS: traditional Unix permissions AND TCC. TCC is enforced at the kernel level. `sudo` bypasses Unix ownership checks but does NOT bypass TCC. A CLI binary must be run from a Terminal app (or shell) that has been granted Full Disk Access in System Settings → Privacy & Security → Full Disk Access.

Additionally, the entitlement propagates from the controlling process: if `freespace` is invoked from iTerm2 and iTerm2 has Full Disk Access, the binary inherits that access. If run from a shell without FDA, it does not.

**How to avoid:**
- Handle `EPERM` (ErrorKind::PermissionDenied) gracefully in the traversal loop: log to stderr, skip the subtree, continue the scan.
- Track how many paths were skipped due to permission errors and report that count in the summary output (e.g., "12 directories skipped — permission denied").
- In the `freespace doctor` command, actively probe known TCC-protected paths (`~/Desktop`, `~/Documents`, `~/Library/Mail`) and report whether Full Disk Access appears to be granted.
- Document in the README that Full Disk Access must be granted to Terminal (or to the binary itself via an entitlement) for accurate results.

**Warning signs:**
- Scan of home directory returns a suspiciously low total (< 5 GB on a typical developer machine)
- No errors reported, but known large directories (Documents, Downloads) are not reflected in totals
- `freespace caches` finds nothing in `~/Library/Caches` despite it being visibly large

**Phase to address:** Scan (milestone 3) for error handling; CLI skeleton (milestone 1) for the `doctor` subcommand scaffolding.

---

### Pitfall 5: Trash Behavior on Non-Home-Volume Files Is Unexpected

**What goes wrong:**
On macOS, "move to Trash" does not mean "move to `~/.Trash`." For files on the same volume as the home directory, they go to `~/.Trash`. For files on a different APFS volume or external disk, they must go to `<volume>/.Trashes/<uid>/` on that same volume — otherwise macOS performs a cross-device copy-then-delete, which can be slow or fail entirely, and it defeats the zero-space-cost expectation of "Trash."

When using the `trash` crate (`Byron/trash-rs`), the macOS implementation calls the Finder/NSWorkspace trash API which handles volume-local Trash correctly. However, if the crate call fails silently or falls back to a manual implementation, files may end up in the wrong location or be permanently deleted instead of trashed.

**Why it happens:**
Developers test cleanup on files in their home directory. External volume edge cases are not covered. The `trash` crate has known limitations: listing Trash contents and restoring from Trash are not fully implemented on macOS (no stable public API exists for this).

**How to avoid:**
- Always use the `trash` crate's `trash::delete()` API — never implement manual move-to-trash logic.
- Before calling delete, verify the path is not on a read-only volume (disk images, network volumes) where Trash may not exist.
- After trashing, log the action to `~/.local/state/Freespace/cleanup.log` including the original path, the volume it was on, and the timestamp.
- For the cleanup preview, show the user which volume each file lives on — this surfaces cross-volume situations.
- Test explicitly with files on an external APFS volume (even a disk image counts).

**Warning signs:**
- Cleanup of external drive files is slow (seconds per file instead of milliseconds) — indicates a cross-device copy rather than an atomic move
- Files "trashed" from a mounted disk image are not visible in Finder's Trash
- The `trash` crate returns an error for files on network volumes (`smb://`, `afp://`)

**Phase to address:** Cleanup Apply (milestone 7) — Trash behavior must be tested before the apply command ships.

---

### Pitfall 6: File Size Uses Logical Length Instead of Allocated Blocks

**What goes wrong:**
`metadata().len()` returns the logical byte count of a file. For sparse files (large files with holes — VM disk images, database files, Docker volumes), the logical size can be orders of magnitude larger than actual disk consumption. A 100 GB sparse `.vmdk` may occupy only 12 GB on disk. Summing logical sizes inflates the apparent disk usage and misleads cleanup decisions.

**Why it happens:**
The POSIX `stat` struct provides both `st_size` (logical bytes) and `st_blocks` (512-byte blocks actually allocated). Rust's `Metadata::len()` maps to `st_size`. Developers reach for the obvious API without considering that APFS supports sparse files and that Docker, Vagrant, and VM tools create them routinely.

**How to avoid:**
Use `MetadataExt::st_blocks()` for physical allocation size when reporting "size on disk":

```rust
use std::os::unix::fs::MetadataExt;

// Physical bytes on disk (st_blocks counts 512-byte units)
let physical = meta.st_blocks() * 512;
// Logical bytes (what the file "thinks" it is)
let logical = meta.len();
```

Report both where the difference is significant (> 10% delta). In the scan output, use physical size as the primary "disk usage" figure — it matches what `du` reports and what users care about when trying to free space.

**Warning signs:**
- Scan of a developer's home directory shows multi-hundred-GB totals that far exceed what macOS Finder or `df` reports
- VM-related directories (`~/.vagrant.d`, `~/VMs`, Docker's `~/Library/Containers/com.docker.docker`) show implausibly large sizes

**Phase to address:** Scan (milestone 3) — the `FileEntry.size` field must be defined as physical (allocated) bytes from the start; changing this later requires recalculating all downstream aggregates.

---

### Pitfall 7: SIP-Protected Paths Must Be Blocked at the Code Level, Not Just by Convention

**What goes wrong:**
The protected path list in config (`/System`, `/usr`, `/bin`, `/sbin`, `/private`) is checked at deletion time. But if a user-supplied path or a resolved symlink points into a SIP-protected area, the scan may traverse it (returning partial data due to TCC denial) and the preview may show files as "safe to delete." The deletion attempt then fails with a confusing error, or — worse — the user has disabled SIP and the deletion succeeds, potentially corrupting the OS.

**Why it happens:**
Protected-path checks are applied to the input path literally. If the scan root is `/` and the traversal reaches `/System`, the check must also catch `/System/Volumes/Data` (the APFS data volume that underlies `~/`), `/private/var`, and symlink-resolved paths like `/etc` → `/private/etc`.

**How to avoid:**
- Maintain a compile-time list of protected path prefixes (not just exact matches).
- Resolve all paths with `std::fs::canonicalize()` before checking against the protected list.
- Block traversal into protected paths entirely, not just deletion. Emitting a warning and skipping is correct; entering them silently is not.
- Add `/System/Volumes` to the blocked list in addition to `/System`.
- In `freespace doctor`, report which of the protected paths are present on the running system.

**Warning signs:**
- The blocked-path check is only applied in the cleanup module, not in the scan module
- Tests do not include a case where a symlink resolves into a protected path
- `/private/var` and `/private/tmp` are not in the protected list even though `/var` and `/tmp` are symlinks to them

**Phase to address:** CLI skeleton (milestone 1) for the protected-path constant list; Scan (milestone 3) to block traversal; Cleanup Apply (milestone 7) as the final hard check.

---

### Pitfall 8: Classification by Extension Produces Wrong Category for macOS-Specific Paths

**What goes wrong:**
The classification module applies path rules before extension rules (correct priority), but the path rule list may be incomplete, leaving macOS-specific paths to fall through to extension matching. For example:

- `~/Library/Caches/com.apple.dt.Xcode` — contains `.db`, `.sqlite`, `.dylib` files → would classify as "documents" or "unknown" without a path rule
- `~/.ollama/models/blobs/` — contains files with no extension or SHA hash names → classifies as "unknown"
- `~/Library/Application Support/Docker/` — contains large `.raw` disk image files → would classify as "images" (wrong)
- `~/Library/Group Containers/` — cloud sync / app group data → falls through to extension matching

**Why it happens:**
The path rule list is manually curated. macOS adds new standard directories with every OS release. Extension rules have no context about whether a `.raw` file is a camera raw image or a disk image.

**How to avoid:**
- Define path rules as the authoritative source of truth. Any path under `~/Library/Caches/**` is always "caches" regardless of extension.
- Define extension rules only for paths that do not match any path rule.
- Use a tiered rule evaluation: (1) exact path match, (2) prefix match for known directories, (3) extension match, (4) "unknown."
- Add tests for every known macOS standard directory that the project targets — include a fixture for each category.
- For Docker and VM paths specifically, add explicit path rules for `~/Library/Containers/com.docker.docker` and `~/.vagrant.d`.

**Warning signs:**
- `freespace categories ~/Library` shows large amounts in "documents" or "unknown" that should be in "caches" or "developer"
- Extension-matched categories account for more than 30% of total classified size on a developer machine
- The `.raw` extension is mapped to "images" without a context check

**Phase to address:** Categories (milestone 4) — classification tests must cover real macOS directory structures before the command ships.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Use `metadata().len()` for all file sizes | Simple, one-line implementation | Sparse files inflate totals; APFS clones appear larger than they are | Never for the primary size field |
| Skip inode deduplication | Simpler traversal loop | Hardlinked files counted multiple times; totals wrong for Docker/Git repos | Never in fs_scan module |
| Use `follow_links(false)` and skip all symlinks | No loop risk | Symlinks not reported to user; hidden size in symlinked paths missed | Acceptable for MVP if symlinks are reported with a count |
| Block protected paths by literal string match only | Fast to implement | Symlinks like `/etc` → `/private/etc` bypass the check | Never for safety-critical blocked-path logic |
| Omit TCC/FDA check in `doctor` subcommand | Less code to ship | Users can't diagnose why scan results are incomplete | Defer to post-MVP only if `doctor` ships later |
| Hard-code category path rules as `match` arms | Readable code | Every new macOS directory requires a code change and recompile | Acceptable for v1; move to config file in v2 |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `trash` crate (Byron/trash-rs) | Assuming all trash operations succeed silently | Always check the `Result`; log failure path and surface to user |
| `trash` crate on external volumes | Expecting same-speed move as home volume | Cross-device files trigger copy+delete; show progress or warn |
| `walkdir` with `follow_links(true)` | Assuming loop detection prevents all hangs | Loop errors must be handled explicitly; `filter_map(ok)` hides them |
| `statvfs` for available space | Treating `f_bavail * f_frsize` as the complete picture | APFS purgeable space is separate; surface both values |
| `std::fs::canonicalize()` on broken symlinks | Panics or returns error unexpectedly | Broken symlinks are a valid state; handle `Err` and continue |
| `MetadataExt` on symlinks | Getting symlink metadata instead of target | `symlink_metadata()` vs `metadata()` behave differently when `follow_links` is false |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Loading all `FileEntry` structs into a `Vec` before aggregating | Memory grows linearly with file count; OOM on large trees | Streaming aggregation — accumulate counts and totals without storing all entries | ~500k files (~200 MB RAM hit) |
| Collecting top-N largest files by sorting the full list | Extremely slow on 100k+ file scans | Use a fixed-size min-heap (`BinaryHeap` with capacity N) — O(n log N) instead of O(n log n) | ~50k files before noticeable lag |
| `HashSet` for inode deduplication with default hasher | Adequate but slow for high-inode directories | Use `AHashMap`/`AHashSet` (faster than `SipHash` default) for the inode tracking set | ~1M inodes (Docker base image layers) |
| Calling `canonicalize()` on every path during traversal | syscall overhead multiplies | Only canonicalize paths being added to cleanup candidates, not every scan entry | ~10k files per second degradation |
| Re-reading directory metadata after scan (for cleanup preview) | Files may have changed; metadata is stale | Store snapshot time in scan results; warn if preview is older than N minutes | Any scan + delayed preview scenario |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Protected-path check only at delete time | A crafted config exclusion or symlink could route deletion into `/System` | Check at both traversal entry and delete time; use `canonicalize()` |
| Logging file paths to `cleanup.log` without sanitizing | Paths containing newlines or control characters corrupt the log | Use structured logging (one JSON object per line) or escape paths |
| Trusting user-supplied `--path` without canonicalization | A relative path like `../../System` passes a naive prefix check | Always canonicalize before comparing against the protected-path list |
| Permanent delete (`--force`) without confirmation prompt | One mistyped command deletes irreplaceable data | Require explicit `--force` AND a `--yes` confirmation flag for non-interactive use |
| Allowing `clean apply` before a scan has been run | Cleanup runs on stale or nonexistent data | Enforce build order at the code level — cleanup requires a valid scan result object, not just a flag |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Reporting "recovered X GB" when APFS clones share blocks | User expects X GB freed; gets much less; loses trust in the tool | Report "up to X GB may be freed — actual recovery depends on APFS clone sharing" |
| Showing size in bytes or raw numbers without context | Power users are fine; non-technical users are confused | Always format as human-readable (GiB/MiB) in table output; raw bytes only in --json |
| No indication of which directories were skipped (TCC denied) | User thinks scan is complete; it is not | Report "N directories skipped (permission denied)" in summary |
| Displaying cleanup preview without a staleness warning | User previews, walks away, returns 10 minutes later, applies | Timestamp the preview; warn if more than 5 minutes old |
| Permanent delete defaulting silently | Users accustomed to Trash are surprised by irrecoverable loss | Make Trash the default; `--force` requires explicit acknowledgement; log every permanent deletion |

---

## "Looks Done But Isn't" Checklist

- [ ] **Hardlink deduplication:** Size totals look correct on a simple test directory — verify with a directory containing hardlinks (`ln` two files to the same inode) that the total does not double-count
- [ ] **Symlink loop handling:** Scanner exits cleanly — verify with `mkdir /tmp/loop && ln -s /tmp/loop /tmp/loop/loop && freespace scan /tmp/loop`
- [ ] **APFS clone accounting caveat:** Size reporting works — verify that output includes a note about potential overcount on APFS volumes
- [ ] **TCC denied paths tracking:** No crash on permission error — verify that denied directories are counted and surfaced in the summary, not silently swallowed
- [ ] **Trash on external volume:** `clean apply` succeeds on home volume — verify that it also works (or fails gracefully) with a file on an APFS disk image
- [ ] **Protected-path via symlink:** `/etc` is blocked — verify that a file at `/private/etc/hosts` is also blocked (since `/etc` → `/private/etc`)
- [ ] **Sparse file physical size:** Large `.vmdk` or `.sparseimage` shows logical vs. physical size — verify that `FileEntry.size` reflects allocated blocks, not logical length
- [ ] **Cleanup before scan:** The `clean apply` command is accessible — verify it refuses to run if no scan results exist in the session

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Hardlink double-counting shipped to users | MEDIUM | Patch `fs_scan` to add inode deduplication; bump minor version; document the fix in changelog so users re-run scans |
| APFS clone overcounting causes user to over-delete | HIGH | Requires restoring from backup; add caveat language to all size displays immediately as a hotfix |
| TCC denial silently skips directories | LOW | Add permission-denied counter to scan summary; release patch; no data loss |
| Trash fails on external volume, files permanently deleted | HIGH | No automated recovery; advise user to check Time Machine; add pre-flight trash test to next release |
| SIP bypass via unresolved symlink | HIGH | Emergency patch to add `canonicalize()` before all protected-path checks; audit all call sites |
| Classification wrong (e.g., disk image classified as "images") | LOW | Update path rule table; release patch; no data loss as long as cleanup requires user confirmation |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Hardlink double-counting | Scan (milestone 3) | Unit test: directory with `ln` hardlinks; assert total equals single-copy size |
| Symlink loop hang | Scan (milestone 3) | Integration test: fixture with symlink cycle; assert clean exit |
| APFS clone accounting caveat | Scan (milestone 3) + Summary (milestone 2) | Output review: check that caveat language appears; verify purgeable space is surfaced |
| TCC permission denial silently skipping paths | Scan (milestone 3) | Integration test: mock `EPERM` error; assert skip counter increments |
| Trash behavior on external volumes | Cleanup Apply (milestone 7) | Manual test: file on mounted disk image; assert atomic move (not copy+delete) |
| File size using logical not physical bytes | Scan (milestone 3) | Unit test: create sparse file; assert `FileEntry.size` equals `st_blocks * 512` |
| SIP-protected path via symlink bypass | CLI skeleton (milestone 1) | Unit test: path `/etc` → resolves to `/private/etc`; assert blocked |
| Classification falling through to wrong extension match | Categories (milestone 4) | Unit tests: one fixture per macOS standard directory; assert correct category |
| Cleanup running before scan | Cleanup Preview + Apply (milestones 6–7) | Integration test: invoke `clean apply` with no prior scan; assert rejection |

---

## Sources

- [walkdir docs — WalkDir struct, follow_links, loop detection](https://docs.rs/walkdir/latest/walkdir/struct.WalkDir.html)
- [Rust Cookbook — Directory Traversal](https://rust-lang-nursery.github.io/rust-cookbook/file/dir.html)
- [Byron/trash-rs — GitHub, macOS implementation limitations (issue #8)](https://github.com/Byron/trash-rs/issues/8)
- [The Eclectic Light Company — Free space on an APFS volume is an illusion](https://eclecticlight.co/2022/12/30/free-space-on-an-apfs-volume-is-an-illusion/)
- [The Eclectic Light Company — APFS: Files and clones (2024)](https://eclecticlight.co/2024/03/20/apfs-files-and-clones/)
- [The Eclectic Light Company — Explainer: Permissions, privacy and TCC](https://eclecticlight.co/2025/11/08/explainer-permissions-privacy-and-tcc/)
- [Apple Support — About System Integrity Protection](https://support.apple.com/en-us/102149)
- [Apple Developer — Disabling and Enabling SIP](https://developer.apple.com/documentation/security/disabling-and-enabling-system-integrity-protection)
- [Apple Developer Forums — Permissions on .Trash folder](https://developer.apple.com/forums/thread/122716)
- [RUSTSEC-2023-0018 — TOCTOU race condition in remove_dir_all](https://rustsec.org/advisories/RUSTSEC-2023-0018.html)
- [erdtree (erd) — hardlink handling in Rust disk tools](https://lib.rs/crates/erdtree)
- [parallel-disk-usage (pdu) — hardlink deduplication](https://lib.rs/crates/parallel-disk-usage)
- [Cargo PR #10214 — resilience to filesystem loop while walking dirs](https://github.com/rust-lang/cargo/pull/10214)
- [macmost.com — How Mac Trash Handles External Drive Files](https://macmost.com/how-mac-trash-handles-external-drive-files.html)
- [DaisyDisk guide — Snapshots and purgeable space](https://daisydiskapp.com/guide/4/en/Snapshots/)
- [osxdaily.com — Fix "Operation not permitted" error in macOS Terminal](https://osxdaily.com/2018/10/09/fix-operation-not-permitted-terminal-error-macos/)

---
*Pitfalls research for: Rust CLI disk utility (macOS) — Freespace*
*Researched: 2026-03-28*
