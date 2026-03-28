use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "freespace",
    about = "macOS disk inspection and cleanup utility",
    version,
    author
)]
pub struct Cli {
    /// Output results as JSON (stdout only; errors remain on stderr)
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show disk usage summary for all mounted volumes
    Summary,
    /// Scan a path and report size breakdown
    Scan {
        path: std::path::PathBuf,
    },
    /// Show largest files and directories at a path
    Largest {
        path: std::path::PathBuf,
    },
    /// Group disk usage into semantic categories at a path
    Categories {
        path: std::path::PathBuf,
    },
    /// List hidden files and directories at a path
    Hidden {
        path: std::path::PathBuf,
    },
    /// Discover cache directories across standard macOS locations
    Caches,
    /// Cleanup operations (preview or apply)
    Clean {
        #[command(subcommand)]
        command: CleanCommands,
    },
    /// Show and edit Freespace configuration
    Config,
    /// Run self-diagnostics
    Doctor,
}

#[derive(Subcommand)]
pub enum CleanCommands {
    /// Show files that would be removed (read-only, no changes made)
    Preview,
    /// Execute cleanup (moves to Trash by default)
    Apply {
        /// Permanently delete instead of moving to Trash
        #[arg(long)]
        force: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn json_flag_is_global() {
        // --json before subcommand
        let cli = Cli::try_parse_from(["freespace", "--json", "summary"])
            .expect("--json before subcommand must parse");
        assert!(cli.json, "--json flag must be true");
    }

    #[test]
    fn json_flag_propagates_to_nested_subcommand() {
        // --json AFTER nested subcommand (the critical global = true test)
        let cli = Cli::try_parse_from(["freespace", "clean", "preview", "--json"])
            .expect("--json after nested subcommand must parse with global = true");
        assert!(cli.json, "--json must propagate through clean > preview");
    }

    #[test]
    fn clean_apply_force_flag() {
        let cli = Cli::try_parse_from(["freespace", "clean", "apply", "--force"])
            .expect("clean apply --force must parse");
        match cli.command {
            Commands::Clean { command: CleanCommands::Apply { force } } => {
                assert!(force, "--force must be true");
            }
            _ => panic!("expected Commands::Clean {{ Apply }}"),
        }
    }

    #[test]
    fn clean_preview_parses() {
        let cli = Cli::try_parse_from(["freespace", "clean", "preview"])
            .expect("clean preview must parse");
        assert!(matches!(
            cli.command,
            Commands::Clean { command: CleanCommands::Preview }
        ));
    }

    #[test]
    fn scan_takes_path_argument() {
        let cli = Cli::try_parse_from(["freespace", "scan", "/tmp"])
            .expect("scan /tmp must parse");
        match cli.command {
            Commands::Scan { path } => assert_eq!(path.to_str().unwrap(), "/tmp"),
            _ => panic!("expected Commands::Scan"),
        }
    }

    #[test]
    fn help_exits_with_display_help_error() {
        let result = Cli::try_parse_from(["freespace", "--help"]);
        assert!(result.is_err(), "--help must produce an error (clap convention)");
        // Extract the error without requiring Debug on Cli (unwrap_err/expect_err need Debug on Ok type)
        match result {
            Err(err) => assert_eq!(
                err.kind(),
                ErrorKind::DisplayHelp,
                "--help must produce DisplayHelp, not a real error"
            ),
            Ok(_) => panic!("--help should have returned an error"),
        }
    }

    #[test]
    fn json_default_is_false() {
        let cli = Cli::try_parse_from(["freespace", "summary"])
            .expect("summary without --json must parse");
        assert!(!cli.json, "--json default must be false");
    }
}
