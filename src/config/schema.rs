use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub scan: ScanConfig,
    #[serde(default)]
    pub cleanup: CleanupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            exclude: vec![
                // Cloud storage virtual FSes report the same device ID as ~
                // but are network-backed and cause timeouts on traversal.
                home.join("Library/CloudStorage").to_string_lossy().into_owned(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CleanupConfig {
    #[serde(default)]
    pub safe_categories: Vec<String>,
}
