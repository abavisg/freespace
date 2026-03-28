use crate::config::schema::Config;

pub fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    let _ = config;
    if json {
        crate::output::write_json(&serde_json::json!({
            "status": "not_implemented",
            "command": "doctor"
        }))?;
    } else {
        eprintln!("doctor: not yet implemented");
    }
    Ok(())
}
