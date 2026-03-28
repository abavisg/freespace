use crate::config::schema::Config;

pub fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    let _ = (config, json);
    eprintln!("summary: not yet implemented");
    Ok(())
}
