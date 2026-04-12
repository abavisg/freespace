use crate::classify::{safety_class, SafetyClass};
use crate::config::schema::Config;
use bytesize::ByteSize;
use comfy_table::Table;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct CacheEntry {
    path: PathBuf,
    total_bytes: u64,
    file_count: u64,
    safety: SafetyClass,
}

#[derive(Debug, Serialize)]
struct CachesResult {
    entries: Vec<CacheEntry>,
    total_cache_bytes: u64,
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

pub fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve home directory"))?;

    let dirs = known_cache_dirs(&home);

    let mut entries: Vec<CacheEntry> = Vec::new();

    for dir in dirs {
        // Skip nonexistent dirs silently (Pitfall 5)
        if !dir.exists() {
            continue;
        }

        let scan_result = crate::fs_scan::scan_path(&dir, config);
        let safety = safety_class(&dir, &home);

        entries.push(CacheEntry {
            path: dir,
            total_bytes: scan_result.total_bytes,
            file_count: scan_result.file_count,
            safety,
        });
    }

    // Sort by total_bytes descending (largest first)
    entries.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));

    let total_cache_bytes: u64 = entries.iter().map(|e| e.total_bytes).sum();
    let reclaimable_bytes: u64 = entries
        .iter()
        .filter(|e| e.safety == SafetyClass::Safe)
        .map(|e| e.total_bytes)
        .sum();

    let result = CachesResult {
        entries,
        total_cache_bytes,
        reclaimable_bytes,
    };

    if json {
        crate::output::write_json(&result)?;
    } else {
        render_caches_table(&result);
    }

    Ok(())
}

fn render_caches_table(result: &CachesResult) {
    let mut table = Table::new();
    table.set_header(vec!["Path", "Size", "Files", "Safety"]);

    for entry in &result.entries {
        let path_str = entry.path.to_string_lossy().to_string();
        let size = ByteSize::b(entry.total_bytes).to_string();
        let files = entry.file_count.to_string();
        let safety = entry.safety.to_string();
        table.add_row(vec![path_str, size, files, safety]);
    }

    println!("{table}");
    println!(
        "Total cache size: {}",
        ByteSize::b(result.total_cache_bytes)
    );
    println!(
        "Reclaimable (safe): {}",
        ByteSize::b(result.reclaimable_bytes)
    );
}
