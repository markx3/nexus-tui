use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nexus", version, about = "Cyberpunk TUI session manager for Claude Code")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,
}
