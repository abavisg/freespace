# freespace Cheatsheet

## The Workflow

```
1. summary       → what volumes do I have?
2. scan          → what's in this path?
3. largest       → what are the biggest offenders?
4. categories    → what type of stuff is it?
5. caches        → what caches can I safely clear?
6. clean preview → what would cleanup actually touch?
7. clean apply   → do it (Trash) / clean apply --force (permanent)
```

---

## Inspect

| Command | What it does |
|---------|-------------|
| `freespace summary` | All mounted volumes — total / used / available |
| `freespace scan <path>` | Total size, file count, dir count, largest items |
| `freespace largest <path>` | Top-N largest files and directories |
| `freespace categories <path>` | Usage by category (video, caches, developer, …) |
| `freespace hidden <path>` | Dotfiles and hidden dirs with sizes |
| `freespace caches` | All cache dirs — size + safety rating |

---

## Cleanup

| Command | What it does |
|---------|-------------|
| `freespace clean preview` | Read-only preview — nothing deleted |
| `freespace clean apply` | Move to Trash (recoverable) |
| `freespace clean apply --force` | **Permanently delete** (irreversible) |

Cleanup log: `~/.local/state/Freespace/cleanup.log`

---

## Diagnose & Configure

| Command | What it does |
|---------|-------------|
| `freespace doctor` | Check FDA, protected paths, config, log |
| `freespace doctor --json` | Same, as JSON (exits 1 on failure) |
| `freespace config` | Show active configuration |

---

## JSON Output

Add `--json` to any command for scriptable output:

```bash
freespace summary --json
freespace scan ~/Downloads --json | jq '.total_size'
freespace caches --json | jq '[.[] | select(.safety == "safe")]'
freespace doctor --json | jq '.overall'
freespace categories ~ --json | jq '.[] | select(.name == "caches")'
```

JSON → stdout. Errors/logs → stderr. Always.

---

## Shell Completions

```bash
# zsh
freespace completions zsh > ~/.zsh/completions/_freespace
# add to ~/.zshrc: fpath=(~/.zsh/completions $fpath)

# bash
freespace completions bash > /usr/local/etc/bash_completion.d/freespace

# fish
freespace completions fish > ~/.config/fish/completions/freespace.fish
```

---

## Configuration

`~/.config/Freespace/config.toml`

```toml
[scan]
exclude = ["node_modules", ".git", "target"]

[cleanup]
safe_categories = ["caches", "developer"]
```

---

## Full Disk Access

Required for ~/Library, iCloud Drive, and protected directories.

```
System Settings → Privacy & Security → Full Disk Access → add freespace
```

Check status: `freespace doctor`

---

## Safety Rules

- Protected paths (`/System /usr /bin /sbin /private`) are **always blocked**
- `clean apply` requires a prior `clean preview` session
- Default deletion = **Trash** (recoverable in Finder)
- `--force` = permanent delete — no confirmation prompt, no undo

---

## Common Recipes

```bash
# What's eating my disk?
freespace summary
freespace scan ~ | head -30

# Find the 20 biggest things in Downloads
freespace largest ~/Downloads

# How much cache can I safely clear?
freespace caches | grep -i safe

# Check before cleaning
freespace clean preview
freespace clean apply

# Scripted: clean only if >5GB reclaimable
SIZE=$(freespace clean preview --json | jq '[.[].size] | add')
[ "$SIZE" -gt 5368709120 ] && freespace clean apply

# Is everything healthy?
freespace doctor && echo "All good"

# Pipe categories into a report
freespace categories ~ --json | jq -r '.[] | "\(.size_human)\t\(.name)"' | sort -rh
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (or doctor: pass/warn) |
| 1 | Error (or doctor: any check failed) |
