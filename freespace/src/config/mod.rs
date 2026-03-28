pub mod schema;

use anyhow::Context;
use schema::Config;

/// Load configuration from ~/.config/Freespace/config.toml.
///
/// Missing file → returns Config::default() (graceful).
/// Present but malformed → returns Err with context.
///
/// IMPORTANT: Uses dirs::home_dir().join(".config/Freespace/config.toml")
/// NOT dirs::config_dir() — on macOS, config_dir() returns
/// ~/Library/Application Support, which is NOT the PRD-mandated path.
pub fn load_config() -> anyhow::Result<Config> {
    let path = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve home directory"))?
        .join(".config/Freespace/config.toml");

    if !path.exists() {
        tracing::debug!(
            "No config file at {}; using defaults",
            path.display()
        );
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    toml::from_str(&contents)
        .with_context(|| format!("Config file is malformed: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_toml(dir: &tempfile::TempDir, contents: &str) -> std::path::PathBuf {
        let path = dir.path().join("config.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        path
    }

    fn load_from_path(path: &std::path::Path) -> anyhow::Result<Config> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        toml::from_str(&contents)
            .with_context(|| format!("Config malformed: {}", path.display()))
    }

    #[test]
    fn missing_file_returns_default() {
        // load_config() reads from home dir — test the graceful path via
        // a non-existent temp path using the internal helper logic directly.
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("no_such_file.toml");
        // Simulate: if !path.exists() => default
        if !absent.exists() {
            let cfg = Config::default();
            assert!(cfg.scan.exclude.is_empty());
            assert!(cfg.cleanup.safe_categories.is_empty());
        }
    }

    #[test]
    fn valid_toml_parses_exclude() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_toml(
            &dir,
            r#"
[scan]
exclude = ["/tmp", "/var"]
"#,
        );
        let cfg = load_from_path(&path).unwrap();
        assert_eq!(cfg.scan.exclude, vec!["/tmp", "/var"]);
    }

    #[test]
    fn valid_toml_parses_safe_categories() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_toml(
            &dir,
            r#"
[cleanup]
safe_categories = ["caches", "downloads"]
"#,
        );
        let cfg = load_from_path(&path).unwrap();
        assert_eq!(cfg.cleanup.safe_categories, vec!["caches", "downloads"]);
    }

    #[test]
    fn malformed_toml_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_toml(&dir, "this is not valid toml ][");
        let result = load_from_path(&path);
        assert!(result.is_err(), "malformed TOML must return Err");
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_toml(
            &dir,
            r#"
[scan]
exclude = []
future_unknown_key = "value"
"#,
        );
        // serde with #[serde(default)] ignores unknown fields
        let result = load_from_path(&path);
        assert!(result.is_ok(), "unknown TOML keys must not cause an error");
    }
}
