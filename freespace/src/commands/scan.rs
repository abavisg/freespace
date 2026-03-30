use crate::config::schema::Config;
use bytesize::ByteSize;
use comfy_table::Table;
use std::path::Path;

fn render_scan_table(result: &crate::analyze::ScanResult, root: &Path) {
    let mut table = Table::new();
    table.set_header(vec!["Path", "Size", "Files", "Dirs", "Skipped"]);
    table.add_row(vec![
        root.display().to_string(),
        ByteSize::b(result.total_bytes).to_string(),
        result.file_count.to_string(),
        result.dir_count.to_string(),
        result.skipped_count.to_string(),
    ]);
    println!("{table}");
}

pub fn run(path: &Path, config: &Config, json: bool) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("path does not exist: {}", path.display());
    }
    let result = crate::fs_scan::scan_path(path, config);
    if json {
        crate::output::write_json(&result)?;
    } else {
        render_scan_table(&result, path);
    }
    Ok(())
}
