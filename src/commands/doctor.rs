use crate::config::schema::Config;
use serde::Serialize;
use std::path::Path;
#[cfg(target_os = "macos")]
use std::path::PathBuf;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
}

#[derive(Debug, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
}

fn check_full_disk_access(home: &Path) -> DoctorCheck {
    let probe = home.join("Library/Safari/History.db");
    match std::fs::metadata(&probe) {
        Ok(_) => DoctorCheck {
            name: "Full Disk Access".into(),
            status: CheckStatus::Pass,
            message: "Granted".into(),
        },
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => DoctorCheck {
            name: "Full Disk Access".into(),
            status: CheckStatus::Fail,
            message: "Open System Settings > Privacy & Security > Full Disk Access and add freespace".into(),
        },
        Err(_) => DoctorCheck {
            name: "Full Disk Access".into(),
            status: CheckStatus::Warn,
            message: "Cannot determine — Safari History.db not present; grant FDA manually to be sure".into(),
        },
    }
}

fn check_protected_paths() -> DoctorCheck {
    #[cfg(target_os = "macos")]
    {
        let paths: Vec<PathBuf> = crate::platform::macos::protected_paths();
        let total = paths.len();
        let verified = paths.iter().filter(|p| p.exists()).count();
        if verified == total {
            DoctorCheck {
                name: "Protected paths".into(),
                status: CheckStatus::Pass,
                message: format!("{verified}/{total} verified"),
            }
        } else {
            let missing: Vec<String> = paths
                .iter()
                .filter(|p| !p.exists())
                .map(|p| p.display().to_string())
                .collect();
            DoctorCheck {
                name: "Protected paths".into(),
                status: CheckStatus::Warn,
                message: format!(
                    "{verified}/{total} verified — unresolved: {}",
                    missing.join(", ")
                ),
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        DoctorCheck {
            name: "Protected paths".into(),
            status: CheckStatus::Warn,
            message: "Not a macOS host — protected path check skipped".into(),
        }
    }
}

fn check_config_file(home: &Path) -> DoctorCheck {
    let config_path = home.join(".config/Freespace/config.toml");
    if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str::<toml::Value>(&contents) {
                Ok(_) => DoctorCheck {
                    name: "Config file".into(),
                    status: CheckStatus::Pass,
                    message: config_path.display().to_string(),
                },
                Err(e) => DoctorCheck {
                    name: "Config file".into(),
                    status: CheckStatus::Fail,
                    message: format!(
                        "{} is not valid TOML: {e} — fix syntax or delete the file to use defaults",
                        config_path.display()
                    ),
                },
            },
            Err(e) => DoctorCheck {
                name: "Config file".into(),
                status: CheckStatus::Warn,
                message: format!(
                    "{} exists but could not be read: {e}",
                    config_path.display()
                ),
            },
        }
    } else {
        DoctorCheck {
            name: "Config file".into(),
            status: CheckStatus::Warn,
            message: format!(
                "{} not found — defaults will be used; create it to customize scan excludes",
                config_path.display()
            ),
        }
    }
}

fn check_cleanup_log(home: &Path) -> DoctorCheck {
    let log_path = home.join(".local/state/Freespace/cleanup.log");
    if log_path.exists() {
        DoctorCheck {
            name: "Cleanup log".into(),
            status: CheckStatus::Pass,
            message: log_path.display().to_string(),
        }
    } else {
        DoctorCheck {
            name: "Cleanup log".into(),
            status: CheckStatus::Warn,
            message: "Not yet created — will be created on first `freespace clean apply` run".into(),
        }
    }
}

use comfy_table::Table;

fn render_doctor_table(checks: &[DoctorCheck]) {
    let mut table = Table::new();
    table.set_header(vec!["Check", "Status", "Message"]);
    for check in checks {
        let symbol = match check.status {
            CheckStatus::Pass => "✓",
            CheckStatus::Fail => "✗",
            CheckStatus::Warn => "⚠",
        };
        table.add_row(vec![
            check.name.clone(),
            symbol.to_string(),
            check.message.clone(),
        ]);
    }
    println!("{table}");
}

pub fn run(config: &Config, json: bool) -> anyhow::Result<()> {
    let _ = config;
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve home directory"))?;

    let checks = vec![
        check_full_disk_access(&home),
        check_protected_paths(),
        check_config_file(&home),
        check_cleanup_log(&home),
    ];

    let fail_count = checks
        .iter()
        .filter(|c| matches!(c.status, CheckStatus::Fail))
        .count();
    let warn_count = checks
        .iter()
        .filter(|c| matches!(c.status, CheckStatus::Warn))
        .count();

    let overall: &'static str = if fail_count > 0 {
        "fail"
    } else if warn_count > 0 {
        "warn"
    } else {
        "pass"
    };

    if json {
        crate::output::write_json(&serde_json::json!({
            "checks": checks,
            "overall": overall,
        }))?;
    } else {
        render_doctor_table(&checks);
        if fail_count == 0 {
            println!("All checks passed");
        } else {
            println!("{fail_count} check(s) failed — see above");
        }
    }

    if fail_count > 0 {
        anyhow::bail!("{fail_count} check(s) failed");
    }
    Ok(())
}
