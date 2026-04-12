use crate::classify::is_hidden;
use crate::config::schema::Config;
use bytesize::ByteSize;
use comfy_table::Table;
use serde::Serialize;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
struct HiddenEntry {
    path: PathBuf,
    size_bytes: u64,
    is_dir: bool,
}

#[derive(Debug, Serialize)]
struct HiddenResult {
    root: PathBuf,
    entries: Vec<HiddenEntry>,
    total_hidden_bytes: u64,
    total_hidden_count: u64,
}

pub fn run(path: &Path, config: &Config, json: bool) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("path does not exist: {}", path.display());
    }

    let mut entries: Vec<HiddenEntry> = Vec::new();

    // Enumerate IMMEDIATE children only (no recursive descent into hidden dirs)
    match std::fs::read_dir(path) {
        Ok(dir_iter) => {
            for entry_result in dir_iter {
                let entry = match entry_result {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!("read_dir entry error at {:?}: {}", path, e);
                        continue;
                    }
                };

                let entry_path = entry.path();

                if !is_hidden(&entry_path) {
                    continue;
                }

                let file_type = match entry.file_type() {
                    Ok(ft) => ft,
                    Err(e) => {
                        tracing::warn!("file_type error at {:?}: {}", entry_path, e);
                        continue;
                    }
                };

                if file_type.is_dir() {
                    // Size hidden directory recursively via scan_path
                    let scan_result = crate::fs_scan::scan_path(&entry_path, config);
                    entries.push(HiddenEntry {
                        path: entry_path,
                        size_bytes: scan_result.total_bytes,
                        is_dir: true,
                    });
                } else if file_type.is_file() {
                    let metadata = match entry.metadata() {
                        Ok(m) => m,
                        Err(e) => {
                            tracing::warn!("metadata error at {:?}: {}", entry_path, e);
                            continue;
                        }
                    };
                    // Physical size: blocks * 512
                    let size_bytes = metadata.blocks() * 512;
                    entries.push(HiddenEntry {
                        path: entry_path,
                        size_bytes,
                        is_dir: false,
                    });
                }
                // Symlinks and other file types: skip
            }
        }
        Err(e) => {
            anyhow::bail!("cannot read directory {:?}: {}", path, e);
        }
    }

    // Sort by size_bytes descending (largest first)
    entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    let total_hidden_bytes: u64 = entries.iter().map(|e| e.size_bytes).sum();
    let total_hidden_count = entries.len() as u64;

    let result = HiddenResult {
        root: path.to_path_buf(),
        entries,
        total_hidden_bytes,
        total_hidden_count,
    };

    if json {
        crate::output::write_json(&result)?;
    } else {
        render_hidden_table(&result);
    }

    Ok(())
}

fn render_hidden_table(result: &HiddenResult) {
    let mut table = Table::new();
    table.set_header(vec!["Name", "Type", "Size"]);

    for entry in &result.entries {
        let name = entry
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
            .to_string();
        let kind = if entry.is_dir { "dir" } else { "file" };
        let size = ByteSize::b(entry.size_bytes).to_string();
        table.add_row(vec![name, kind.to_string(), size]);
    }

    println!("Hidden items in: {}", result.root.display());
    println!("{table}");
    println!(
        "Total: {} hidden items, {}",
        result.total_hidden_count,
        ByteSize::b(result.total_hidden_bytes)
    );
}
