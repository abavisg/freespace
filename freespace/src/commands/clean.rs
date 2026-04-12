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

    let dirs_list = known_cache_dirs(&home);

    let mut candidates: Vec<PreviewEntry> = Vec::new();
    for dir in &dirs_list {
        if !dir.exists() {
            continue;
        }
        let scan = crate::fs_scan::scan_path(dir, config);
        let safety = safety_class(dir, &home);
        candidates.push(PreviewEntry {
            path: dir.clone(),
            total_bytes: scan.total_bytes,
            file_count: scan.file_count,
            safety,
        });
    }

    // Sort: primary key safety ascending (Safe first), secondary key total_bytes descending
    candidates.sort_by(|a, b| {
        a.safety
            .cmp(&b.safety)
            .then(b.total_bytes.cmp(&a.total_bytes))
    });

    let total_bytes: u64 = candidates.iter().map(|e| e.total_bytes).sum();
    let reclaimable_bytes: u64 = candidates
        .iter()
        .filter(|e| e.safety == SafetyClass::Safe)
        .map(|e| e.total_bytes)
        .sum();

    let result = PreviewResult {
        candidates,
        total_bytes,
        reclaimable_bytes,
    };

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
    table.set_header(["Path", "Size", "Files", "Safety"]);
    for entry in &result.candidates {
        table.add_row([
            entry.path.to_string_lossy().to_string(),
            ByteSize::b(entry.total_bytes).to_string(),
            entry.file_count.to_string(),
            entry.safety.to_string(),
        ]);
    }
    println!("{table}");
    println!("Total: {}", ByteSize::b(result.total_bytes));
    println!("Reclaimable (safe): {}", ByteSize::b(result.reclaimable_bytes));
}

pub fn run_apply(force: bool, _config: &Config, json: bool) -> anyhow::Result<()> {
    let session = load_preview_session()?;
    let protected = crate::platform::macos::protected_paths();
    let network_mounts = network_mount_points();
    let count = session.candidates.len();
    let total_bytes: u64 = session.candidates.iter().map(|c| c.total_bytes).sum();

    // Confirmation gate (skipped in --json mode).
    if !json {
        print!(
            "{} items, {} — Proceed? [y/N] ",
            count,
            bytesize::ByteSize::b(total_bytes)
        );
        std::io::stdout().flush()?;
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        if !line.trim().eq_ignore_ascii_case("y") {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    let sdir = state_dir()?;
    std::fs::create_dir_all(&sdir)?;

    let mut trashed = 0usize;
    let mut deleted = 0usize;
    let mut skipped = 0usize;

    for entry in &session.candidates {
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

        if force {
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
            // Use NsFileManager method: faster, no osascript serialization,
            // and does not require Finder permissions. Trade-off: "Put Back"
            // may not appear in Finder context menu on some macOS versions.
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
