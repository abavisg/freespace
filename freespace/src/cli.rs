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

    #[test]
    fn all_subcommands_compile() {
        // Verifies the CLI struct compiles with all required subcommands.
        // Full --help output tests are in Wave 1.
        let _ = std::mem::size_of::<Cli>();
        let _ = std::mem::size_of::<Commands>();
        let _ = std::mem::size_of::<CleanCommands>();
    }
}
