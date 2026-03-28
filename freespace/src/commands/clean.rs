use crate::config::schema::Config;

pub fn run_preview(config: &Config, json: bool) -> anyhow::Result<()> {
    let _ = config;
    if json {
        crate::output::write_json(&serde_json::json!({
            "status": "not_implemented",
            "command": "clean preview"
        }))?;
    } else {
        eprintln!("clean preview: not yet implemented");
    }
    Ok(())
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
