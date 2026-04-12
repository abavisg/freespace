use crate::classify::{classify_path, Category};
use crate::config::schema::Config;
use bytesize::ByteSize;
use comfy_table::Table;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Serialize)]
struct CategoryEntry {
    category: Category,
    total_bytes: u64,
    file_count: u64,
}

#[derive(Debug, Serialize)]
struct CategoriesResult {
    root: PathBuf,
    categories: Vec<CategoryEntry>,
    total_bytes: u64,
    total_files: u64,
}

pub fn run(path: &Path, config: &Config, json: bool) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("path does not exist: {}", path.display());
    }

    // Resolve home once, outside the walk loop
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

    // Pre-initialize all 14 categories with zero counts (ensures all show up in output)
    let mut totals: HashMap<Category, (u64, u64)> = HashMap::new();
    for cat in Category::all() {
        totals.insert(*cat, (0u64, 0u64));
    }

    // Hardlink deduplication via (dev, ino)
    let mut seen_inodes: HashSet<(u64, u64)> = HashSet::new();

    for entry_result in WalkDir::new(path).follow_links(false) {
        match entry_result {
            Ok(entry) => {
                // Skip configured exclusions
                if config
                    .scan
                    .exclude
                    .iter()
                    .any(|ex| entry.path().starts_with(ex))
                {
                    continue;
                }

                if !entry.file_type().is_file() {
                    continue;
                }

                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!("metadata error at {:?}: {}", entry.path(), e);
                        continue;
                    }
                };

                // Deduplicate hardlinks
                let key = (metadata.dev(), metadata.ino());
                if !seen_inodes.insert(key) {
                    // Already counted this inode
                    continue;
                }

                // Physical size: blocks * 512
                let physical = metadata.blocks() * 512;

                let cat = classify_path(entry.path(), &home);
                let entry_totals = totals.entry(cat).or_insert((0, 0));
                entry_totals.0 += physical;
                entry_totals.1 += 1;
            }
            Err(e) => {
                tracing::warn!("scan error: {}", e);
            }
        }
    }

    // Build result in the stable order of Category::all()
    let mut categories: Vec<CategoryEntry> = Category::all()
        .iter()
        .map(|cat| {
            let (total_bytes, file_count) = totals[cat];
            CategoryEntry {
                category: *cat,
                total_bytes,
                file_count,
            }
        })
        .collect();

    // Sort by total_bytes descending for table readability (keep JSON stable)
    let total_bytes: u64 = categories.iter().map(|e| e.total_bytes).sum();
    let total_files: u64 = categories.iter().map(|e| e.file_count).sum();

    if json {
        // For JSON, sort by Category::all() order (already in that order)
        let result = CategoriesResult {
            root: path.to_path_buf(),
            categories,
            total_bytes,
            total_files,
        };
        crate::output::write_json(&result)?;
    } else {
        // For table, sort by total_bytes descending
        categories.sort_by(|a, b| b.total_bytes.cmp(&a.total_bytes));
        render_categories_table(&categories, path, total_bytes, total_files);
    }

    Ok(())
}

fn render_categories_table(
    categories: &[CategoryEntry],
    root: &Path,
    total_bytes: u64,
    total_files: u64,
) {
    let mut table = Table::new();
    table.set_header(vec!["Category", "Size", "Files"]);

    for entry in categories {
        table.add_row(vec![
            entry.category.to_string(),
            ByteSize::b(entry.total_bytes).to_string(),
            entry.file_count.to_string(),
        ]);
    }

    // Summary row
    table.add_row(vec![
        "TOTAL".to_string(),
        ByteSize::b(total_bytes).to_string(),
        total_files.to_string(),
    ]);

    println!("Categories for: {}", root.display());
    println!("{table}");
}
