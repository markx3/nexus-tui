use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "nexus", version, about = "Cyberpunk TUI session manager for Claude Code")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Output in JSON format (for subcommands that support it)
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all sessions
    List,
    /// Show details for a specific session
    Show {
        /// Session ID (or prefix)
        session_id: String,
    },
    /// Launch a session in tmux
    Launch {
        /// Session ID
        session_id: String,
    },
    /// Kill a tmux session
    Kill {
        /// Session name in tmux
        session_name: String,
    },
    /// Scan for Claude Code sessions
    Scan,
    /// List configured groups
    Groups,
}
