use crate::config::schema::Config;

pub fn run_preview(config: &Config, json: bool) -> anyhow::Result<()> {
    let _ = (config, json);
    eprintln!("clean preview: not yet implemented");
    Ok(())
}

pub fn run_apply(force: bool, config: &Config, json: bool) -> anyhow::Result<()> {
    let _ = (force, config, json);
    eprintln!("clean apply: not yet implemented");
    Ok(())
}
