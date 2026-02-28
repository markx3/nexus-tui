mod app;
mod cli;
pub(crate) mod config;
pub(crate) mod db;
pub(crate) mod grouping;
#[cfg(test)]
mod mock;
pub(crate) mod scanner;
mod text_utils;
mod theme;
mod time_utils;
pub(crate) mod tmux;
pub(crate) mod types;
mod ui;
pub(crate) mod widgets;

use clap::Parser;
use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = cli::Cli::parse();

    if let Some(command) = cli.command {
        run_cli(command, cli.json)
    } else {
        run_tui()
    }
}

fn run_cli(command: cli::Commands, json: bool) -> Result<()> {
    let config = config::load_config()?;
    let db = db::Database::open(&config.general.db_path)?;

    match command {
        cli::Commands::List => {
            let tree = db.get_tree()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&tree)?);
            } else {
                print_tree(&tree, 0);
            }
        }
        cli::Commands::Show { session_id } => {
            let tree = db.get_tree()?;
            let session = find_session_in_tree(&tree, &session_id);
            match session {
                Some(s) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(s)?);
                    } else {
                        print_session_detail(s);
                    }
                }
                None => {
                    color_eyre::eyre::bail!("Session '{}' not found", session_id);
                }
            }
        }
        cli::Commands::Launch { session_id } => {
            let _lock = acquire_lock()?;
            let tmux = tmux::TmuxManager::new(&config.tmux.socket_name);
            if !tmux.is_available() {
                color_eyre::eyre::bail!("tmux is not available");
            }
            let cwd = db
                .get_session_cwd(&session_id)?
                .ok_or_else(|| color_eyre::eyre::eyre!("Session '{}' has no cwd", session_id))?;
            let name = sanitize_tmux_name(&session_id);
            tmux.launch_session(&name, &cwd)?;
            println!("Launched session '{}'", session_id);
        }
        cli::Commands::Kill { session_name } => {
            let _lock = acquire_lock()?;
            let tmux = tmux::TmuxManager::new(&config.tmux.socket_name);
            tmux.kill_window(&session_name)?;
            println!("Killed session '{}'", session_name);
        }
        cli::Commands::Scan => {
            let scan_result = scanner::scan_quick(&config.general.projects_dir)?;
            db.upsert_sessions(&scan_result.sessions)?;
            let tree = db.get_tree()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&tree)?);
            } else {
                println!("Scanned {} sessions", scan_result.sessions.len());
                if !scan_result.warnings.is_empty() {
                    for w in &scan_result.warnings {
                        eprintln!("  warn: {w}");
                    }
                }
                print_tree(&tree, 0);
            }
        }
        cli::Commands::Groups => {
            let tree = db.get_tree()?;
            let groups: Vec<&types::GroupNode> = tree
                .iter()
                .filter_map(|n| {
                    if let types::TreeNode::Group(g) = n {
                        Some(g)
                    } else {
                        None
                    }
                })
                .collect();
            if json {
                println!("{}", serde_json::to_string_pretty(&groups)?);
            } else {
                for g in &groups {
                    println!("{} ({} sessions)", g.name, g.children.len());
                }
            }
        }
    }
    Ok(())
}

fn run_tui() -> Result<()> {
    let _lock = acquire_lock()?;
    let config = config::load_config()?;
    let db = db::Database::open(&config.general.db_path)?;

    for group_def in &config.groups {
        let icon = if group_def.icon.is_empty() {
            "◈"
        } else {
            &group_def.icon
        };
        if let Err(e) = db.create_group(&group_def.name, icon) {
            // UNIQUE constraint violation means group already exists -- ignore
            if !e.to_string().contains("UNIQUE") {
                return Err(e);
            }
        }
    }

    let scan_result = scanner::scan_quick(&config.general.projects_dir)?;
    db.upsert_sessions(&scan_result.sessions)?;

    if !config.auto_group.is_empty() {
        grouping::apply_rules(&config.auto_group, &db)?;
    }

    let tree = db.get_tree()?;
    let tmux = tmux::TmuxManager::new(&config.tmux.socket_name);
    let tmux_available = tmux.is_available();
    if tmux_available {
        if let Err(e) = tmux.setup_keybindings() {
            eprintln!("Warning: failed to setup tmux keybindings: {e}");
        }
    }

    let tmux_windows = if tmux_available {
        tmux.list_windows().unwrap_or_default()
    } else {
        vec![]
    };

    let terminal = ratatui::init();
    let result =
        app::App::new(config, tree, tmux, tmux_available, tmux_windows).run(terminal);
    ratatui::restore();
    result
}

fn find_session_in_tree<'a>(
    tree: &'a [types::TreeNode],
    id: &str,
) -> Option<&'a types::SessionSummary> {
    for node in tree {
        match node {
            types::TreeNode::Session(s) => {
                if s.session_id == id || s.session_id.starts_with(id) {
                    return Some(s);
                }
            }
            types::TreeNode::Group(g) => {
                if let Some(s) = find_session_in_tree(&g.children, id) {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn sanitize_tmux_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect()
}

fn print_tree(tree: &[types::TreeNode], depth: usize) {
    let indent = "  ".repeat(depth);
    for node in tree {
        match node {
            types::TreeNode::Group(g) => {
                println!("{indent}# {} ({} sessions)", g.name, g.children.len());
                print_tree(&g.children, depth + 1);
            }
            types::TreeNode::Session(s) => {
                let status = if s.is_active { "+" } else { "-" };
                println!("{indent}{status} {} [{}]", s.display_name, s.last_active);
            }
        }
    }
}

fn print_session_detail(s: &types::SessionSummary) {
    println!("Session: {}", s.session_id);
    println!("Name:    {}", s.display_name);
    println!("Project: {}", s.project_dir);
    if let Some(ref cwd) = s.cwd {
        println!("CWD:     {}", cwd.display());
    }
    if let Some(ref branch) = s.git_branch {
        println!("Branch:  {}", branch);
    }
    if let Some(ref model) = s.model {
        println!("Model:   {}", model);
    }
    println!("Messages: {}", s.message_count);
    println!("Tokens:  {} in / {} out", s.input_tokens, s.output_tokens);
    println!("Active:  {}", s.is_active);
    println!("Last:    {}", s.last_active);
}

fn acquire_lock() -> Result<fslock::LockFile> {
    let lock_dir = dirs::cache_dir()
        .ok_or_else(|| {
            color_eyre::eyre::eyre!(
                "Cannot determine cache directory. Set XDG_CACHE_HOME or HOME."
            )
        })?
        .join("nexus");
    std::fs::create_dir_all(&lock_dir)?;

    let mut lock = fslock::LockFile::open(&lock_dir.join("nexus.lock"))?;

    if !lock.try_lock()? {
        color_eyre::eyre::bail!(
            "Another nexus instance is already running.\n  \
             If this is a stale lock, remove: {}",
            lock_dir.join("nexus.lock").display()
        );
    }

    Ok(lock)
}
