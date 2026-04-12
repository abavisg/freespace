use crate::config::schema::Config;
use crate::output;
#[cfg(target_os = "macos")]
use crate::platform::macos;
use comfy_table::Table;

#[cfg(target_os = "macos")]
fn render_table(volumes: &[macos::VolumeInfo]) {
    use bytesize::ByteSize;
    let mut table = Table::new();
    table.set_header(vec!["Mount Point", "Total", "Used", "Available"]);
    for v in volumes {
        table.add_row(vec![
            v.mount_point.clone(),
            ByteSize::b(v.total_bytes).to_string(),
            ByteSize::b(v.used_bytes).to_string(),
            ByteSize::b(v.available_bytes).to_string(),
        ]);
    }
    println!("{table}");
}

pub fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    let _ = config;
    #[cfg(target_os = "macos")]
    {
        let volumes = macos::list_volumes();
        if json {
            output::write_json(&volumes)?;
        } else {
            render_table(&volumes);
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("summary: not supported on this platform");
    }
    Ok(())
}
