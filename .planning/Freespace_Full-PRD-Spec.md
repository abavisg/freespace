# Freespace
## Full PRD + Engineering Spec

## 1. Overview

Freespace is a terminal-first Rust CLI for inspecting, categorising, and safely reclaiming disk space on macOS.

It is designed for power users who want:

* clear visibility into disk usage
* deterministic categorisation
* safe cleanup workflows
* scriptable outputs

---

## 2. Core Philosophy

This tool follows a strict sequence:

Inspect → Classify → Preview → Clean

If this order is violated, the tool becomes unsafe.

---

## 3. Product Goals

* Fast and accurate disk inspection
* Deterministic categorisation
* Safe, explainable cleanup
* Terminal-native UX
* JSON output for automation

---

## 4. Non-Goals

* Exact replication of macOS Storage UI
* Automatic cleanup without preview
* Deep system-level integrations (Photos, Mail internals)

---

## 5. Target Users

* Developers
* Technical power users
* Engineers managing large local datasets

---

## 6. MVP Features

### 6.1 Summary

Command:
Freespace summary

Outputs:

* mounted volumes
* total, used, available space

---

### 6.2 Scan

Command:
Freespace scan <path>

Outputs:

* total size
* file count
* directory count
* largest files
* largest directories

---

### 6.3 Categories

Command:
Freespace categories <path>

Outputs:

* grouped disk usage by category

Categories:

* video
* audio
* images
* documents
* archives
* applications
* developer
* caches
* mail
* containers
* cloud-sync
* hidden
* system-related
* unknown

---

### 6.4 Hidden

Command:
Freespace hidden <path>

Outputs:

* dotfiles
* hidden directories
* total hidden size

---

### 6.5 Caches

Command:
Freespace caches

Outputs:

* cache directories
* reclaimable size
* safety classification

---

### 6.6 Cleanup Preview

Command:
Freespace clean preview

Outputs:

* files to be affected
* reclaimable space
* safety classification

---

### 6.7 Cleanup Apply

Command:
Freespace clean apply

Rules:

* default = move to Trash
* permanent delete requires --force
* protected paths cannot be deleted

---

## 7. CLI Design

Freespace summary
Freespace scan <path>
Freespace largest <path>
Freespace categories <path>
Freespace hidden <path>
Freespace caches
Freespace clean preview
Freespace clean apply
Freespace config
Freespace doctor

---

## 8. Safety Model

Safety classes:

* safe
* caution
* dangerous
* blocked

Rules:

* no deletion without explicit input
* no system path deletion
* Trash preferred over permanent delete

Protected paths:

* /System
* /usr
* /bin
* /sbin
* /private

---

## 9. Architecture

Modules:

cli

* argument parsing
* command routing

fs_scan

* directory traversal
* metadata collection

classify

* path-based rules
* extension rules

analyze

* aggregation
* top-N logic

cleanup

* preview generation
* deletion execution

config

* user config

output

* tables
* JSON

platform::macos

* macOS-specific behavior

---

## 10. Data Models

VolumeInfo

* mount_point
* total_bytes
* used_bytes
* available_bytes

FileEntry

* path
* size
* is_dir
* is_hidden
* category

CategoryTotal

* category
* total_bytes

CleanupCandidate

* path
* size
* safety

---

## 11. Classification Rules

Priority:

1. Path rules
2. Known macOS dirs
3. Extension mapping
4. Fallback unknown

Examples:
~/Library/Caches → caches
~/Library/Mail → mail
~/.ollama → developer

---

## 12. Config

Path:
~/.config/Freespace/config.toml

Example:

[scan]
exclude = ["/System"]

[cleanup]
safe_categories = ["caches"]

---

## 13. JSON Support

All major commands support --json

Rules:

* clean JSON only on stdout
* logs go to stderr

---

## 14. Performance

* must handle large directories
* avoid loading everything in memory
* use streaming aggregation

---

## 15. Error Handling

Handle:

* permission denied
* broken symlinks
* deleted during scan

Do not crash. Continue when safe.

---

## 16. Logging

Log cleanup actions to:
~/.local/state/Freespace/cleanup.log

---

## 17. Implementation Stack

Rust crates:

* clap
* walkdir
* serde
* toml
* trash
* comfy-table

---

## 18. Project Structure

src/
main.rs
cli.rs
commands/
fs/
classify/
analyze/
cleanup/
config/
output/
platform/

---

## 19. Milestones

1. CLI skeleton
2. summary
3. scan
4. categories
5. hidden + caches
6. cleanup preview
7. cleanup apply
8. JSON + config

---

## 20. Testing

Unit:

* classification
* config
* safety rules

Integration:

* fake directory trees
* hidden files
* symlinks

---

## 21. AI Strategy (Future Only)

AI is NOT part of MVP.

Future optional use:

* cleanup suggestions
* edge-case classification

Constraints:

* opt-in only
* advisory only
* never auto-delete

---

## 22. Success Criteria

* accurate reporting
* safe cleanup
* predictable behaviour
* stable performance

---

## 23. Final Instruction for Claude Code

Build in this order:

1. summary
2. scan
3. categories
4. hidden
5. caches
6. preview
7. cleanup

Never implement cleanup before scan and classification are reliable.

This is a safety-critical rule, not a suggestion.
