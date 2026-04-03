use crate::config::schema::Config;
use bytesize::ByteSize;
use comfy_table::Table;
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::fs_scan::FileEntry;

#[derive(Debug, Serialize)]
struct LargestResult {
    root: PathBuf,
    total_bytes: u64,
    largest_files: Vec<FileEntry>,
    largest_dirs: Vec<FileEntry>,
}

pub fn run(path: &Path, config: &Config, json: bool) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("path does not exist: {}", path.display());
    }
    let result = crate::fs_scan::scan_path(path, config);

    if json {
        let output = LargestResult {
            root: result.root,
            total_bytes: result.total_bytes,
            largest_files: result.largest_files,
            largest_dirs: result.largest_dirs,
        };
        crate::output::write_json(&output)?;
    } else {
        render_largest_table(&result.largest_files, &result.largest_dirs, path, result.total_bytes);
    }
    Ok(())
}

fn render_largest_table(files: &[FileEntry], dirs: &[FileEntry], root: &Path, total_bytes: u64) {
    println!("Largest items in: {} (total: {})", root.display(), ByteSize::b(total_bytes));
    println!();

    if !files.is_empty() {
        let mut table = Table::new();
        table.set_header(vec!["#", "Size", "Path"]);
        for (i, f) in files.iter().enumerate() {
            table.add_row(vec![
                (i + 1).to_string(),
                ByteSize::b(f.size).to_string(),
                f.path.display().to_string(),
            ]);
        }
        println!("Largest Files:");
        println!("{table}");
        println!();
    }

    if !dirs.is_empty() {
        let mut table = Table::new();
        table.set_header(vec!["#", "Size", "Path"]);
        for (i, d) in dirs.iter().enumerate() {
            table.add_row(vec![
                (i + 1).to_string(),
                ByteSize::b(d.size).to_string(),
                d.path.display().to_string(),
            ]);
        }
        println!("Largest Directories:");
        println!("{table}");
    }
}
