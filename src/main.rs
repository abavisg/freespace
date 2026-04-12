mod analyze;
mod classify;
mod cli;
mod commands;
mod config;
mod fs_scan;
mod output;
mod platform;

use anyhow::Context;
use clap::Parser;
use cli::{CleanCommands, Commands};

fn main() -> anyhow::Result<()> {
    init_logging();
    let cli = cli::Cli::parse();
    let config = config::load_config().context("Failed to load configuration")?;
    #[cfg(target_os = "macos")]
    let _protected = platform::macos::protected_paths();
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
    match cli.command {
        Commands::Summary => commands::summary::run(&config, cli.json),
        Commands::Scan { path } => commands::scan::run(&path.unwrap_or_else(|| home.clone()), &config, cli.json),
        Commands::Largest { path } => commands::largest::run(&path.unwrap_or_else(|| home.clone()), &config, cli.json),
        Commands::Categories { path } => commands::categories::run(&path.unwrap_or_else(|| home.clone()), &config, cli.json),
        Commands::Hidden { path } => commands::hidden::run(&path.unwrap_or_else(|| home.clone()), &config, cli.json),
        Commands::Caches => commands::caches::run(&config, cli.json),
        Commands::Clean { command } => match command {
            CleanCommands::Preview => commands::clean::run_preview(&config, cli.json),
            CleanCommands::Apply { force } => commands::clean::run_apply(force, &config, cli.json),
        },
        Commands::Config => commands::config_cmd::run(&config, cli.json),
        Commands::Doctor => commands::doctor::run(&config, cli.json),
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            use clap_complete::generate;
            let mut cmd = cli::Cli::command();
            generate(shell, &mut cmd, "freespace", &mut std::io::stdout());
            Ok(())
        }
    }
}

fn init_logging() {
    use tracing_subscriber::{fmt, EnvFilter};
    fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_target(false)
        .init();
}
