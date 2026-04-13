use crate::classify::{safety_class, SafetyClass};
use crate::config::schema::Config;
use bytesize::ByteSize;
use comfy_table::Table;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PreviewEntry {
    path: PathBuf,
    total_bytes: u64,
    file_count: u64,
    safety: SafetyClass,
}

#[derive(Debug, Serialize)]
struct PreviewResult {
    candidates: Vec<PreviewEntry>,
    total_bytes: u64,
    reclaimable_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PreviewSession {
    timestamp: u64,
    candidates: Vec<PreviewEntry>,
}

fn state_dir() -> anyhow::Result<std::path::PathBuf> {
    if let Ok(override_dir) = std::env::var("FREESPACE_STATE_DIR") {
        return Ok(std::path::PathBuf::from(override_dir));
    }
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve home directory"))?;
    Ok(home.join(".local/state/Freespace"))
}

fn write_preview_session(candidates: &[PreviewEntry]) -> anyhow::Result<()> {
    let dir = state_dir()?;
    std::fs::create_dir_all(&dir)?;
    let session = PreviewSession {
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        candidates: candidates.to_vec(),
    };
    let tmp = dir.join("preview-session.json.tmp");
    let final_path = dir.join("preview-session.json");
    let json = serde_json::to_string_pretty(&session)?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &final_path)?;
    Ok(())
}

fn load_preview_session() -> anyhow::Result<PreviewSession> {
    let path = state_dir()?.join("preview-session.json");
    if !path.exists() {
        anyhow::bail!("No preview session found. Run `freespace clean preview` first.");
    }
    let json = std::fs::read_to_string(&path)?;
    let session: PreviewSession = serde_json::from_str(&json)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let age = now.saturating_sub(session.timestamp);
    if age > 3600 {
        anyhow::bail!(
            "Preview session expired ({} minutes ago). Run `freespace clean preview` again.",
            age / 60
        );
    }
    Ok(session)
}

#[derive(Debug, Serialize, Deserialize)]
struct AuditEntry {
    timestamp: String,
    path: String,
    size_bytes: u64,
    action: String, // "trash" | "delete" | "skip"
}

fn utc_timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn append_audit_log(state_dir: &std::path::Path, entry: &AuditEntry) -> anyhow::Result<()> {
    use std::fs::OpenOptions;
    std::fs::create_dir_all(state_dir)?;
    let log_path = state_dir.join("cleanup.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    writeln!(file, "{}", serde_json::to_string(entry)?)?;
    Ok(())
}

fn log_action(state_dir: &std::path::Path, path: &std::path::Path, size_bytes: u64, action: &str) {
    let entry = AuditEntry {
        timestamp: utc_timestamp(),
        path: path.display().to_string(),
        size_bytes,
        action: action.to_string(),
    };
    if let Err(e) = append_audit_log(state_dir, &entry) {
        tracing::warn!("audit log write failed: {}", e);
    }
}

fn network_mount_points() -> HashSet<std::path::PathBuf> {
    use sysinfo::Disks;
    const NETWORK_FS_TYPES: &[&str] = &["smbfs", "afpfs", "nfs", "webdav", "ftpfs", "ftp", "nfs4"];
    let disks = Disks::new_with_refreshed_list();
    let mut mounts: HashSet<std::path::PathBuf> = disks
        .list()
        .iter()
        .filter(|d| {
            let fs = d.file_system().to_string_lossy().to_ascii_lowercase();
            NETWORK_FS_TYPES.iter().any(|n| fs == *n)
        })
        .map(|d| d.mount_point().to_owned())
        .collect();
    // Test hook: treat FREESPACE_FAKE_NETWORK_MOUNT as a network mount for integration tests.
    if let Ok(fake) = std::env::var("FREESPACE_FAKE_NETWORK_MOUNT") {
        mounts.insert(std::path::PathBuf::from(fake));
    }
    mounts
}

/// Returns true if `path` is a directory whose *contents* should be deleted
/// rather than the directory itself. macOS protects these roots and will
/// refuse to trash them even with full disk access.
fn is_cache_root(path: &Path) -> bool {
    let home = dirs::home_dir().unwrap_or_default();
    let roots = [
        home.join("Library/Caches"),
        home.join("Library/Logs"),
    ];
    roots.iter().any(|r| path == r)
}

fn known_cache_dirs(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join("Library/Caches"),
        home.join("Library/Logs"),
        home.join("Library/Developer/Xcode/DerivedData"),
        home.join(".npm"),
        home.join(".cargo/registry"),
        home.join("Library/Containers/com.docker.docker"),
        home.join(".gradle/caches"),
    ]
}

pub fn run_preview(config: &Config, json: bool) -> anyhow::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot resolve home directory"))?;

    let mut candidates: Vec<PreviewEntry> = Vec::new();

    // Section 1: known cache/log dirs
    for dir in &known_cache_dirs(&home) {
        if !dir.exists() {
            continue;
        }
        let scan = crate::fs_scan::scan_path(dir, config);
        if scan.total_bytes == 0 {
            continue;
        }
        candidates.push(PreviewEntry {
            path: dir.clone(),
            total_bytes: scan.total_bytes,
            file_count: scan.file_count,
            safety: safety_class(dir, &home),
        });
    }

    // Section 2: top large directories from a home scan (excluding already-listed paths)
    let already: std::collections::HashSet<PathBuf> = candidates.iter().map(|c| c.path.clone()).collect();
    let home_scan = crate::fs_scan::scan_path(&home, config);
    for dir_entry in &home_scan.largest_dirs {
        // Skip if already in candidates or if it's an ancestor/descendant of one
        if already.iter().any(|p| dir_entry.path.starts_with(p) || p.starts_with(&dir_entry.path)) {
            continue;
        }
        // Skip home itself
        if dir_entry.path == home {
            continue;
        }
        if dir_entry.size < 50 * 1024 * 1024 {
            // Only show dirs >= 50 MB
            continue;
        }
        candidates.push(PreviewEntry {
            path: dir_entry.path.clone(),
            total_bytes: dir_entry.size,
            file_count: 0, // not tracked per-dir in scan
            safety: safety_class(&dir_entry.path, &home),
        });
    }

    // Sort: Safe first, then by size descending
    candidates.sort_by(|a, b| {
        a.safety.cmp(&b.safety).then(b.total_bytes.cmp(&a.total_bytes))
    });

    let total_bytes: u64 = candidates.iter().map(|e| e.total_bytes).sum();
    let reclaimable_bytes: u64 = candidates
        .iter()
        .filter(|e| e.safety == SafetyClass::Safe)
        .map(|e| e.total_bytes)
        .sum();

    let result = PreviewResult { candidates, total_bytes, reclaimable_bytes };

    if let Err(e) = write_preview_session(&result.candidates) {
        tracing::warn!("Could not write preview session file: {}", e);
    }

    if json {
        crate::output::write_json(&result)?;
    } else {
        render_preview_table(&result);
    }
    Ok(())
}

fn render_preview_table(result: &PreviewResult) {
    let mut table = Table::new();
    table.set_header(["#", "Path", "Size", "Files", "Safety"]);
    for (i, entry) in result.candidates.iter().enumerate() {
        let files = if entry.file_count > 0 {
            entry.file_count.to_string()
        } else {
            "—".to_string()
        };
        table.add_row([
            (i + 1).to_string(),
            entry.path.to_string_lossy().to_string(),
            ByteSize::b(entry.total_bytes).to_string(),
            files,
            entry.safety.to_string(),
        ]);
    }
    println!("{table}");
    println!();
    println!("Total shown:        {}", ByteSize::b(result.total_bytes));
    println!("Reclaimable (safe): {}", ByteSize::b(result.reclaimable_bytes));
    println!();
    println!("Run `freespace clean apply` to select which items to delete.");
}

pub fn run_apply(force: bool, _config: &Config, json: bool) -> anyhow::Result<()> {
    let session = load_preview_session()?;
    let protected = crate::platform::macos::protected_paths();
    let network_mounts = network_mount_points();

    // In JSON mode: delete all safe candidates without interaction.
    // In interactive mode: show numbered list and let user pick.
    let selected: Vec<&PreviewEntry> = if json {
        session.candidates.iter()
            .filter(|c| c.safety == SafetyClass::Safe)
            .collect()
    } else {
        // Show numbered list
        println!("Select items to delete (from last preview):");
        println!();
        let mut table = Table::new();
        table.set_header(["#", "Path", "Size", "Safety"]);
        for (i, c) in session.candidates.iter().enumerate() {
            table.add_row([
                (i + 1).to_string(),
                c.path.to_string_lossy().to_string(),
                bytesize::ByteSize::b(c.total_bytes).to_string(),
                c.safety.to_string(),
            ]);
        }
        println!("{table}");
        println!();
        println!("Enter numbers to delete (e.g. 1 3 5), \"safe\" for all safe items, or \"all\" for everything:");
        print!("> ");
        std::io::stdout().flush()?;

        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        let input = line.trim();

        if input.is_empty() || input.eq_ignore_ascii_case("none") {
            eprintln!("Nothing selected. Aborted.");
            return Ok(());
        }

        let chosen: Vec<&PreviewEntry> = if input.eq_ignore_ascii_case("all") {
            session.candidates.iter().collect()
        } else if input.eq_ignore_ascii_case("safe") {
            session.candidates.iter().filter(|c| c.safety == SafetyClass::Safe).collect()
        } else {
            let mut chosen = Vec::new();
            for token in input.split_whitespace() {
                match token.parse::<usize>() {
                    Ok(n) if n >= 1 && n <= session.candidates.len() => {
                        chosen.push(&session.candidates[n - 1]);
                    }
                    _ => {
                        eprintln!("Ignoring invalid selection: {token}");
                    }
                }
            }
            chosen
        };

        if chosen.is_empty() {
            eprintln!("Nothing selected. Aborted.");
            return Ok(());
        }

        // Final confirmation
        let sel_bytes: u64 = chosen.iter().map(|c| c.total_bytes).sum();
        print!(
            "\nDelete {} item(s), {}? [y/N] ",
            chosen.len(),
            bytesize::ByteSize::b(sel_bytes)
        );
        std::io::stdout().flush()?;
        let mut confirm = String::new();
        std::io::stdin().lock().read_line(&mut confirm)?;
        if !confirm.trim().eq_ignore_ascii_case("y") {
            eprintln!("Aborted.");
            return Ok(());
        }
        chosen
    };

    let count = selected.len();
    let total_bytes: u64 = selected.iter().map(|c| c.total_bytes).sum();

    let sdir = state_dir()?;
    std::fs::create_dir_all(&sdir)?;

    let mut trashed = 0usize;
    let mut deleted = 0usize;
    let mut skipped = 0usize;

    for entry in &selected {
        let canonical = std::fs::canonicalize(&entry.path)
            .unwrap_or_else(|_| entry.path.clone());

        // Protected check (runs even with --force).
        if crate::platform::macos::is_protected(&canonical, &protected) {
            tracing::warn!("blocked: protected path — {}", entry.path.display());
            eprintln!("blocked: protected path — {}", entry.path.display());
            log_action(&sdir, &entry.path, entry.total_bytes, "skip");
            skipped += 1;
            continue;
        }

        // Network volume check.
        if network_mounts.iter().any(|mp| {
            canonical.starts_with(mp) || entry.path.starts_with(mp)
        }) {
            eprintln!("skipped: network volume — {}", entry.path.display());
            log_action(&sdir, &entry.path, entry.total_bytes, "skip");
            skipped += 1;
            continue;
        }

        // Existence check (file may have been moved/deleted since preview).
        if !entry.path.exists() {
            tracing::warn!("missing: {}", entry.path.display());
            log_action(&sdir, &entry.path, entry.total_bytes, "skip");
            skipped += 1;
            continue;
        }

        // For directories that macOS won't let us trash/delete as a whole
        // (e.g. ~/Library/Caches itself), delete their children individually.
        // We detect this by checking if the path is a known cache root.
        let children_only = is_cache_root(&entry.path);

        if children_only && entry.path.is_dir() {
            let read_dir = match std::fs::read_dir(&entry.path) {
                Ok(rd) => rd,
                Err(e) => {
                    tracing::warn!("cannot read dir {}: {}", entry.path.display(), e);
                    log_action(&sdir, &entry.path, entry.total_bytes, "skip");
                    skipped += 1;
                    continue;
                }
            };
            let mut child_ok = 0usize;
            let mut child_fail = 0usize;
            for child_entry in read_dir.flatten() {
                let child = child_entry.path();
                if force {
                    let res = if child.is_dir() {
                        std::fs::remove_dir_all(&child)
                    } else {
                        std::fs::remove_file(&child)
                    };
                    match res {
                        Ok(()) => { log_action(&sdir, &child, 0, "delete"); child_ok += 1; }
                        Err(e) => { tracing::warn!("delete failed for {}: {}", child.display(), e); child_fail += 1; }
                    }
                } else {
                    use trash::macos::{DeleteMethod, TrashContextExtMacos};
                    let mut ctx = trash::TrashContext::default();
                    ctx.set_delete_method(DeleteMethod::NsFileManager);
                    match ctx.delete(&child) {
                        Ok(()) => { log_action(&sdir, &child, 0, "trash"); child_ok += 1; }
                        Err(e) => { tracing::warn!("trash failed for {}: {}", child.display(), e); child_fail += 1; }
                    }
                }
            }
            if child_ok > 0 {
                if force { deleted += 1; } else { trashed += 1; }
            }
            if child_fail > 0 { skipped += 1; }
        } else if force {
            let res = if entry.path.is_dir() {
                std::fs::remove_dir_all(&entry.path)
            } else {
                std::fs::remove_file(&entry.path)
            };
            match res {
                Ok(()) => {
                    log_action(&sdir, &entry.path, entry.total_bytes, "delete");
                    deleted += 1;
                }
                Err(e) => {
                    tracing::warn!("delete failed for {}: {}", entry.path.display(), e);
                    log_action(&sdir, &entry.path, entry.total_bytes, "skip");
                    skipped += 1;
                }
            }
        } else {
            use trash::macos::{DeleteMethod, TrashContextExtMacos};
            let mut ctx = trash::TrashContext::default();
            ctx.set_delete_method(DeleteMethod::NsFileManager);
            match ctx.delete(&entry.path) {
                Ok(()) => {
                    log_action(&sdir, &entry.path, entry.total_bytes, "trash");
                    trashed += 1;
                }
                Err(e) => {
                    tracing::warn!("trash failed for {}: {}", entry.path.display(), e);
                    log_action(&sdir, &entry.path, entry.total_bytes, "skip");
                    skipped += 1;
                }
            }
        }
    }

    if json {
        crate::output::write_json(&serde_json::json!({
            "status": "ok",
            "items": count,
            "total_bytes": total_bytes,
            "trashed": trashed,
            "deleted": deleted,
            "skipped": skipped,
        }))?;
    } else {
        println!(
            "Done. trashed={} deleted={} skipped={} total={}",
            trashed,
            deleted,
            skipped,
            bytesize::ByteSize::b(total_bytes)
        );
    }
    Ok(())
}
