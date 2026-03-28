use crate::config::schema::Config;
use std::path::Path;

pub fn run(path: &Path, config: &Config, json: bool) -> anyhow::Result<()> {
    let _ = (path, config, json);
    eprintln!("largest: not yet implemented");
    Ok(())
}
