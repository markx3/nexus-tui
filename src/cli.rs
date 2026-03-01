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
    List {
        /// Include dead/past sessions
        #[arg(long)]
        all: bool,
    },
    /// Show details for a specific session
    Show {
        /// Session ID (or prefix)
        session_id: String,
    },
    /// Create a new session
    New {
        /// Session name
        name: String,
        /// Working directory (defaults to current dir)
        #[arg(short, long)]
        cwd: Option<String>,
        /// Assign to group (created if it doesn't exist)
        #[arg(short, long)]
        group: Option<String>,
    },
    /// Launch/resume a session in tmux
    Launch {
        /// Session ID
        session_id: String,
    },
    /// Kill a tmux session
    Kill {
        /// Session name in tmux
        session_name: String,
    },
    /// List configured groups
    Groups,
}
