mod app;
mod cli;
mod theme;
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
        .unwrap_or_else(std::env::temp_dir)
        .join("nexus");
    std::fs::create_dir_all(&lock_dir)?;

    let mut lock = fslock::LockFile::open(&lock_dir.join("nexus.lock"))?;

    if !lock.try_lock()? {
        eprintln!("nexus: another instance is already running");
        eprintln!(
            "  If this is a stale lock, remove: {}",
            lock_dir.join("nexus.lock").display()
        );
        std::process::exit(1);
    }

    Ok(lock)
}
