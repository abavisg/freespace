# freespace

A terminal-first macOS disk inspection and cleanup utility. Go from zero knowledge to safe, informed disk cleanup in a single session — no surprises, no accidental deletions.

```
freespace summary
freespace scan ~/Downloads
freespace clean preview
freespace clean apply
```

## Features

- **Inspect** — volume summary, recursive path scan, largest files/dirs
- **Classify** — 14-category semantic grouping, hidden file listing, cache discovery with safety ratings
- **Preview** — read-only view of what cleanup would affect before anything is touched
- **Clean** — Trash-first deletion, permanent delete behind `--force`, protected paths immutably blocked
- **Diagnose** — self-check for Full Disk Access, protected paths, config, and cleanup log
- **Script** — every command supports `--json` for clean stdout output

## Installation

### From source (requires Rust 1.75+)

```bash
git clone https://github.com/abavisg/freespace.git
cd freespace/freespace
cargo install --path .
```

### Verify

```bash
freespace --version
# freespace 0.1.0
```

## Quick Start

```bash
# 1. See what's taking up space
freespace summary
freespace scan ~/Downloads

# 2. Understand what's there
freespace largest ~/Downloads
freespace categories ~/Downloads
freespace caches

# 3. Preview what would be cleaned
freespace clean preview

# 4. Clean it
freespace clean apply
```

## Commands

### `summary`

Lists all mounted volumes with total, used, and available space.

```bash
freespace summary
freespace summary --json
```

```
+-------------+---------+---------+-----------+
| Mount       | Total   | Used    | Available |
+============================================+
| /           | 460 GB  | 312 GB  | 148 GB    |
| /Volumes/T7 | 1.0 TB  | 423 GB  | 577 GB    |
+-------------+---------+---------+-----------+
```

---

### `scan <path>`

Scans a path and reports total size, file count, directory count, and largest items. Uses physical disk allocation (not logical file size), deduplicates hardlinks.

```bash
freespace scan ~/Downloads
freespace scan ~/Library --json
```

---

### `largest <path>`

Shows the top largest files and directories at a path.

```bash
freespace largest ~/Downloads
freespace largest / --json
```

---

### `categories <path>`

Groups disk usage into 14 semantic categories: video, audio, images, documents, archives, applications, developer, caches, mail, containers, cloud-sync, hidden, system-related, unknown.

```bash
freespace categories ~/Downloads
freespace categories ~ --json
```

---

### `hidden <path>`

Lists dotfiles and hidden directories with individual sizes and a total.

```bash
freespace hidden ~
freespace hidden ~/Library --json
```

---

### `caches`

Discovers cache directories across standard macOS locations, with path, size, and safety classification (safe / caution / dangerous / blocked) per entry. Shows total reclaimable space across safe caches.

```bash
freespace caches
freespace caches --json
```

---

### `clean preview`

Read-only view of everything cleanup would affect — safety classification and reclaimable size per item. **Makes no changes to disk.**

```bash
freespace clean preview
freespace clean preview --json
```

---

### `clean apply`

Executes cleanup. Moves files to macOS Trash by default (recoverable from Finder). Requires a prior `clean preview` session.

```bash
freespace clean apply            # move to Trash (safe, recoverable)
freespace clean apply --force    # permanently delete (irreversible)
```

Protected paths (`/System`, `/usr`, `/bin`, `/sbin`, `/private`) are blocked under all circumstances. Every action is logged to `~/.local/state/Freespace/cleanup.log`.

---

### `doctor`

Runs self-diagnostics and reports actionable remediation for any issues found.

```bash
freespace doctor
freespace doctor --json
```

Checks:
- **Full Disk Access** — TCC/FDA status via Safari History.db probe
- **Protected paths** — verifies all protected paths resolve correctly
- **Config file** — validates `~/.config/Freespace/config.toml` if present
- **Cleanup log** — confirms log file location

Exits 0 on pass/warn, exits 1 on any failure (scriptable).

---

### `config`

Shows the active configuration.

```bash
freespace config
freespace config --json
```

---

### `completions <shell>`

Generates shell completion scripts.

```bash
freespace completions zsh   > ~/.zsh/completions/_freespace
freespace completions bash  > /usr/local/etc/bash_completion.d/freespace
freespace completions fish  > ~/.config/fish/completions/freespace.fish
```

Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`.

---

## Global Flag

Every command accepts `--json`:

```bash
freespace scan ~/Downloads --json | jq '.total_size'
freespace doctor --json | jq '.overall'
freespace caches --json | jq '[.[] | select(.safety == "safe")] | length'
```

JSON is written to stdout only. Logs and errors always go to stderr.

---

## Configuration

Create `~/.config/Freespace/config.toml` to customise behaviour:

```toml
[scan]
exclude = [
  "node_modules",
  ".git",
  "target",
]

[cleanup]
safe_categories = ["caches", "developer"]
```

If the file is absent, built-in defaults are used. Run `freespace doctor` to validate your config.

---

## Safety Model

freespace enforces a strict workflow order:

```
Inspect → Classify → Preview → Clean
```

- `clean apply` cannot run without a prior `clean preview` session
- Protected paths (`/System`, `/usr`, `/bin`, `/sbin`, `/private`) cannot be deleted under any circumstances
- Trash is the default — permanent deletion requires `--force`
- All cleanup actions are logged with timestamp, path, size, and action type

---

## Full Disk Access

For `freespace scan` and `freespace doctor` to work correctly on protected directories (~/Library, ~/Desktop, iCloud Drive), grant Full Disk Access:

**System Settings → Privacy & Security → Full Disk Access → add `freespace`**

Run `freespace doctor` to verify FDA status.

---

## Building from Source

```bash
git clone https://github.com/abavisg/freespace.git
cd freespace/freespace

# Build
cargo build --release

# Run tests
cargo test

# Install
cargo install --path .
```

**Requirements:** Rust 1.75+, macOS 12+

---

## License

MIT
