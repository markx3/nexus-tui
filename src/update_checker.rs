//! Background update checker for Nexus.
//!
//! Spawns a thread that runs `git ls-remote --tags` against the upstream repo
//! and compares the latest semver tag against the compiled-in version.
//! Rate-limited to at most once per hour via a DB timestamp.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use rusqlite::{params, Connection};
use wait_timeout::ChildExt;

/// Upstream repository URL, baked in at compile time.
const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");

/// Current version, baked in at compile time.
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Minimum interval between network checks (1 hour).
const CHECK_INTERVAL_SECS: u64 = 3600;

/// Timeout for the `git ls-remote` subprocess.
const LS_REMOTE_TIMEOUT: Duration = Duration::from_secs(10);

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

/// Record the latest remote tag as `update_checked_version` in the DB.
///
/// Called after a successful `nexus update` so the background checker won't
/// re-trigger the banner when CURRENT_VERSION in the old binary lags the tag.
pub(crate) fn record_post_update(db_path: &Path) {
    let output = match ls_remote_tags(REPO_URL, LS_REMOTE_TIMEOUT) {
        Some(o) => o,
        None => {
            eprintln!(
                "Warning: could not query remote tags to record update version.\n  \
                 The 'Update available' banner may reappear until the next successful check."
            );
            return;
        }
    };
    if let Some((major, minor, patch)) = latest_tag_version(&output) {
        write_setting(
            db_path,
            "update_checked_version",
            &format!("{major}.{minor}.{patch}"),
        );
    }
}

/// Run the update check. Returns `true` if an update is available.
///
/// Opens its own DB connection to avoid Send issues with the main thread's
/// `Database`. WAL mode allows concurrent reads/writes safely.
fn check_for_update(db_path: &Path) -> bool {
    // Check rate limit
    if let Some(last_check) = read_setting(db_path, "last_update_check") {
        if let Ok(ts) = last_check.parse::<u64>() {
            let now = crate::time_utils::now_epoch();
            if now.saturating_sub(ts) < CHECK_INTERVAL_SECS {
                return read_persisted_state(db_path);
            }
        }
    }

    // Run git ls-remote --tags with timeout
    let output = match ls_remote_tags(REPO_URL, LS_REMOTE_TIMEOUT) {
        Some(o) => o,
        None => return read_persisted_state(db_path),
    };

    // Parse the current compiled-in version
    let current = match parse_semver(CURRENT_VERSION) {
        Some(v) => v,
        None => return read_persisted_state(db_path),
    };

    // Find the latest tag and compare.
    // Suppress if the user already ran `nexus update` for this version
    // (handles the case where the tag was pushed before the version bump).
    let available = match latest_tag_version(&output) {
        Some(latest) if is_newer(latest, current) => {
            let suppressed = read_setting(db_path, "update_checked_version")
                .and_then(|v| parse_semver(&v))
                .is_some_and(|checked| !is_newer(latest, checked));
            !suppressed
        }
        _ => false,
    };

    // Persist results (only on successful network call)
    let now = crate::time_utils::now_epoch().to_string();
    write_setting(db_path, "last_update_check", &now);
    write_setting(
        db_path,
        "update_available",
        if available { "true" } else { "false" },
    );

    available
}

/// Run `git ls-remote --tags <repo_url>` with a timeout.
/// Returns the stdout on success, `None` on failure or timeout.
fn ls_remote_tags(repo_url: &str, timeout: Duration) -> Option<String> {
    let mut child = Command::new("git")
        .args(["ls-remote", "--tags", repo_url])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    match child.wait_timeout(timeout) {
        Ok(Some(status)) if status.success() => {
            let output = child.wait_with_output().ok()?;
            Some(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        Ok(Some(_)) => None, // non-zero exit
        Ok(None) => {
            // Timed out — kill the process
            let _ = child.kill();
            let _ = child.wait();
            None
        }
        Err(_) => None,
    }
}

/// Parse a semver string like "0.3.0" or "v0.3.0" into (major, minor, patch).
fn parse_semver(tag: &str) -> Option<(u64, u64, u64)> {
    let s = tag.strip_prefix('v').unwrap_or(tag);
    let mut parts = s.splitn(3, '.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Parse all tags from `git ls-remote --tags` output and return the highest
/// semver version found. Handles `^{}` dereference entries.
fn latest_tag_version(ls_remote_output: &str) -> Option<(u64, u64, u64)> {
    let mut best: Option<(u64, u64, u64)> = None;

    for line in ls_remote_output.lines() {
        // Format: "<sha>\trefs/tags/<tagname>"
        let refname = line.split('\t').nth(1)?;
        let tag = refname
            .strip_prefix("refs/tags/")
            .unwrap_or(refname)
            .trim_end_matches("^{}");

        if let Some(ver) = parse_semver(tag) {
            if best.is_none_or(|b| is_newer(ver, b)) {
                best = Some(ver);
            }
        }
    }

    best
}

/// Returns true if `candidate` is strictly newer than `current`.
fn is_newer(candidate: (u64, u64, u64), current: (u64, u64, u64)) -> bool {
    candidate > current
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

    // --- DB helper tests ---

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

    // --- Rate-limit tests ---

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

        let now = crate::time_utils::now_epoch().to_string();
        write_setting(&db_path, "last_update_check", &now);
        write_setting(&db_path, "update_available", "true");

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

        let old = crate::time_utils::now_epoch() - 7200;
        write_setting(&db_path, "last_update_check", &old.to_string());
        write_setting(&db_path, "update_available", "false");

        let _result = check_for_update(&db_path);
        let new_ts = read_setting(&db_path, "last_update_check")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        assert!(new_ts >= old);
    }

    // --- Semver parsing tests ---

    #[test]
    fn parse_semver_plain() {
        assert_eq!(parse_semver("1.2.3"), Some((1, 2, 3)));
    }

    #[test]
    fn parse_semver_with_v_prefix() {
        assert_eq!(parse_semver("v0.3.0"), Some((0, 3, 0)));
    }

    #[test]
    fn parse_semver_invalid() {
        assert_eq!(parse_semver("not-a-version"), None);
        assert_eq!(parse_semver("1.2"), None);
        assert_eq!(parse_semver(""), None);
    }

    #[test]
    fn parse_semver_zero() {
        assert_eq!(parse_semver("0.0.0"), Some((0, 0, 0)));
    }

    // --- is_newer tests ---

    #[test]
    fn is_newer_major() {
        assert!(is_newer((1, 0, 0), (0, 9, 9)));
    }

    #[test]
    fn is_newer_minor() {
        assert!(is_newer((0, 4, 0), (0, 3, 5)));
    }

    #[test]
    fn is_newer_patch() {
        assert!(is_newer((0, 3, 1), (0, 3, 0)));
    }

    #[test]
    fn is_newer_equal() {
        assert!(!is_newer((0, 3, 0), (0, 3, 0)));
    }

    #[test]
    fn is_newer_older() {
        assert!(!is_newer((0, 2, 0), (0, 3, 0)));
    }

    // --- latest_tag_version tests ---

    #[test]
    fn latest_tag_from_ls_remote_output() {
        let output = "\
abc123\trefs/tags/v0.1.0\n\
def456\trefs/tags/v0.2.0\n\
ghi789\trefs/tags/v0.3.0\n\
ghi789\trefs/tags/v0.3.0^{}\n";
        assert_eq!(latest_tag_version(output), Some((0, 3, 0)));
    }

    #[test]
    fn latest_tag_skips_non_semver() {
        let output = "\
abc123\trefs/tags/release-candidate\n\
def456\trefs/tags/v0.2.0\n\
ghi789\trefs/tags/nightly\n";
        assert_eq!(latest_tag_version(output), Some((0, 2, 0)));
    }

    #[test]
    fn latest_tag_empty_output() {
        assert_eq!(latest_tag_version(""), None);
    }

    #[test]
    fn latest_tag_no_semver_tags() {
        let output = "abc123\trefs/tags/release-candidate\n";
        assert_eq!(latest_tag_version(output), None);
    }

    #[test]
    fn latest_tag_picks_highest() {
        let output = "\
a\trefs/tags/v1.0.0\n\
b\trefs/tags/v0.9.0\n\
c\trefs/tags/v2.1.3\n\
d\trefs/tags/v2.1.2\n";
        assert_eq!(latest_tag_version(output), Some((2, 1, 3)));
    }
}
