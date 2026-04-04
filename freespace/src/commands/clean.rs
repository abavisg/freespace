use crate::classify::{safety_class, SafetyClass};
use crate::config::schema::Config;
use bytesize::ByteSize;
use comfy_table::Table;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
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

pub fn run_apply(force: bool, config: &Config, json: bool) -> anyhow::Result<()> {
    let _ = (force, config);
    if json {
        crate::output::write_json(&serde_json::json!({
            "status": "not_implemented",
            "command": "clean apply"
        }))?;
    } else {
        eprintln!("clean apply: not yet implemented");
    }
    Ok(())
}
