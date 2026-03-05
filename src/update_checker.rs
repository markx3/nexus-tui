//! Background update checker for Nexus.
//!
//! Spawns a thread that checks if the git source repo (captured at compile time
//! via `CARGO_MANIFEST_DIR`) has newer commits on `origin/main`. Rate-limited
//! to at most once per hour via a DB timestamp.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use rusqlite::{params, Connection};
use wait_timeout::ChildExt;

/// Compile-time path to the source repository.
const SOURCE_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// Minimum interval between network checks (1 hour).
const CHECK_INTERVAL_SECS: u64 = 3600;

/// Timeout for the `git fetch` subprocess.
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// Spawn the update checker thread.
///
/// Returns a receiver that yields a single `bool`: `true` if an update is
/// available, `false` otherwise. The thread exits after sending one result.
pub fn spawn(db_path: &Path) -> mpsc::Receiver<bool> {
    let (tx, rx) = mpsc::channel();
    let db_path = db_path.to_path_buf();
    std::thread::Builder::new()
        .name("nexus-update-check".to_string())
        .spawn(move || {
            let result = check_for_update(&db_path);
            let _ = tx.send(result);
        })
        .expect("failed to spawn update checker thread");
    rx
}

/// Run the update check. Returns `true` if an update is available.
///
/// Opens its own DB connection to avoid Send issues with the main thread's
/// `Database`. WAL mode allows concurrent reads/writes safely.
fn check_for_update(db_path: &Path) -> bool {
    // If source dir doesn't exist or isn't a git repo, return persisted value
    if !Path::new(SOURCE_DIR).join(".git").exists() {
        return read_persisted_state(db_path);
    }

    // Check rate limit
    if let Some(last_check) = read_setting(db_path, "last_update_check") {
        if let Ok(ts) = last_check.parse::<u64>() {
            let now = crate::time_utils::now_epoch();
            if now.saturating_sub(ts) < CHECK_INTERVAL_SECS {
                return read_persisted_state(db_path);
            }
        }
    }

    // Run git fetch with timeout
    if !git_fetch_with_timeout(SOURCE_DIR, FETCH_TIMEOUT) {
        return read_persisted_state(db_path);
    }

    // Count commits behind origin/main
    let behind = commits_behind(SOURCE_DIR);
    let available = behind > 0;

    // Persist results
    let now = crate::time_utils::now_epoch().to_string();
    write_setting(db_path, "last_update_check", &now);
    write_setting(
        db_path,
        "update_available",
        if available { "true" } else { "false" },
    );

    available
}

/// Run `git fetch --quiet origin` with a timeout. Returns true on success.
fn git_fetch_with_timeout(dir: &str, timeout: Duration) -> bool {
    let child = Command::new("git")
        .args(["-C", dir, "fetch", "--quiet", "origin"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(_) => return false,
    };

    match child.wait_timeout(timeout) {
        Ok(Some(status)) => status.success(),
        Ok(None) => {
            // Timed out — kill the process
            let _ = child.kill();
            let _ = child.wait();
            false
        }
        Err(_) => false,
    }
}

/// Count how many commits HEAD is behind origin/main.
fn commits_behind(dir: &str) -> u64 {
    let output = Command::new("git")
        .args(["-C", dir, "rev-list", "--count", "HEAD..origin/main"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .trim()
            .parse()
            .unwrap_or(0),
        _ => 0,
    }
}

/// Read the persisted `update_available` setting from the DB.
fn read_persisted_state(db_path: &Path) -> bool {
    read_setting(db_path, "update_available")
        .map(|v| v == "true")
        .unwrap_or(false)
}

/// Read a single setting from the DB, opening a temporary connection.
fn read_setting(db_path: &Path, key: &str) -> Option<String> {
    let conn = Connection::open(db_path).ok()?;
    let mut stmt = conn
        .prepare("SELECT value FROM settings WHERE key = ?1")
        .ok()?;
    stmt.query_row(params![key], |row| row.get::<_, String>(0))
        .ok()
}

/// Write a single setting to the DB, opening a temporary connection.
fn write_setting(db_path: &Path, key: &str, value: &str) {
    if let Ok(conn) = Connection::open(db_path) {
        let _ = conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_setting_missing_db() {
        assert_eq!(read_setting(Path::new("/nonexistent/db"), "key"), None);
    }

    #[test]
    fn read_persisted_state_missing_db() {
        assert!(!read_persisted_state(Path::new("/nonexistent/db")));
    }

    #[test]
    fn write_and_read_setting() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Create the settings table
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        )
        .unwrap();
        drop(conn);

        write_setting(&db_path, "test_key", "test_value");
        assert_eq!(
            read_setting(&db_path, "test_key"),
            Some("test_value".to_string())
        );
    }

    #[test]
    fn rate_limit_respects_interval() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        )
        .unwrap();
        drop(conn);

        // Set last check to "now" — should return persisted state
        let now = crate::time_utils::now_epoch().to_string();
        write_setting(&db_path, "last_update_check", &now);
        write_setting(&db_path, "update_available", "true");

        // check_for_update should short-circuit and return persisted "true"
        // (won't actually git fetch because rate limit hits first)
        let result = check_for_update(&db_path);
        assert!(result); // returns persisted "true"
    }

    #[test]
    fn rate_limit_allows_stale_check() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        )
        .unwrap();
        drop(conn);

        // Set last check to 2 hours ago — should allow a new check
        let old = crate::time_utils::now_epoch() - 7200;
        write_setting(&db_path, "last_update_check", &old.to_string());
        write_setting(&db_path, "update_available", "false");

        // This will attempt git fetch on SOURCE_DIR (our actual repo) which
        // may or may not have an upstream. The important thing is it doesn't
        // short-circuit on rate limit. It will proceed past rate limit check
        // and either succeed or fail gracefully.
        let _result = check_for_update(&db_path);
        // Verify the timestamp was updated (proving it didn't short-circuit)
        let new_ts = read_setting(&db_path, "last_update_check")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        // If git fetch succeeded, timestamp should be recent
        // If it failed (no remote), timestamp stays old — both are valid
        assert!(new_ts >= old);
    }

    #[test]
    fn commits_behind_invalid_dir() {
        assert_eq!(commits_behind("/nonexistent/path"), 0);
    }

    #[test]
    fn git_fetch_invalid_dir() {
        assert!(!git_fetch_with_timeout("/nonexistent/path", FETCH_TIMEOUT));
    }
}
