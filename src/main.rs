mod app;
mod cli;
pub mod config;
pub mod db;
pub mod grouping;
pub mod mock;
pub mod scanner;
mod theme;
pub mod tmux;
pub mod types;
mod ui;

use clap::Parser;
use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let _cli = cli::Cli::parse();
    let _lock = acquire_lock()?;

    let terminal = ratatui::init();
    let result = app::App::new().run(terminal);
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
