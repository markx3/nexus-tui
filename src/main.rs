mod app;
mod cli;
pub mod config;
pub mod db;
pub mod grouping;
pub mod mock;
pub mod scanner;
mod text_utils;
mod theme;
mod time_utils;
pub mod tmux;
pub mod types;
mod ui;
pub mod widgets;

use clap::Parser;
use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let _cli = cli::Cli::parse();
    let _lock = acquire_lock()?;

    // Load config (defaults if missing)
    let config = config::load_config()?;

    // Init database
    let db = db::Database::open(&config.general.db_path)?;
    db.init_schema()?;

    // Pre-defined groups from config
    for group_def in &config.groups {
        let icon = if group_def.icon.is_empty() {
            "◈"
        } else {
            &group_def.icon
        };
        // Ignore duplicate errors — group may already exist
        let _ = db.create_group(&group_def.name, icon);
    }

    // Scan sessions
    let scan_result = scanner::scan_quick(&config.general.projects_dir)?;
    db.upsert_sessions(&scan_result.sessions)?;

    // Apply auto-grouping rules
    if !config.auto_group.is_empty() {
        grouping::apply_rules(&config.auto_group, &db)?;
    }

    // Build tree from DB
    let tree = db.get_tree()?;

    // Check tmux availability
    let tmux = tmux::TmuxManager::new(&config.tmux.socket_name);
    let tmux_available = tmux.is_available();
    if tmux_available {
        let _ = tmux.setup_keybindings();
    }

    // Initial tmux windows
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
