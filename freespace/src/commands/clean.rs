use crate::classify::{safety_class, SafetyClass};
use crate::config::schema::Config;
use bytesize::ByteSize;
use comfy_table::Table;
use serde::{Deserialize, Serialize};
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
    let _ = force;
    let _session = load_preview_session()?;
    // Full pipeline implemented in Task 3.
    if json {
        crate::output::write_json(&serde_json::json!({
            "status": "session_ok",
            "command": "clean apply"
        }))?;
    } else {
        eprintln!("clean apply: session loaded; pipeline pending");
    }
    Ok(())
}
