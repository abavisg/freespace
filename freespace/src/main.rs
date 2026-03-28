mod cli;
mod commands;
mod config;
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
    match cli.command {
        Commands::Summary => commands::summary::run(&config, cli.json),
        Commands::Scan { path } => commands::scan::run(&path, &config, cli.json),
        Commands::Largest { path } => commands::largest::run(&path, &config, cli.json),
        Commands::Categories { path } => commands::categories::run(&path, &config, cli.json),
        Commands::Hidden { path } => commands::hidden::run(&path, &config, cli.json),
        Commands::Caches => commands::caches::run(&config, cli.json),
        Commands::Clean { command } => match command {
            CleanCommands::Preview => commands::clean::run_preview(&config, cli.json),
            CleanCommands::Apply { force } => commands::clean::run_apply(force, &config, cli.json),
        },
        Commands::Config => commands::config_cmd::run(&config, cli.json),
        Commands::Doctor => commands::doctor::run(&config, cli.json),
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
