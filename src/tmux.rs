use std::process::Command;

use color_eyre::eyre::{bail, WrapErr};
use color_eyre::Result;

use crate::types::{TmuxSessionInfo, TmuxSessionStatus};

// ---------------------------------------------------------------------------
// TmuxManager
// ---------------------------------------------------------------------------

pub struct TmuxManager {
    socket_name: String,
}

impl TmuxManager {
    pub fn new(socket_name: &str) -> Self {
        Self {
            socket_name: socket_name.to_string(),
        }
    }

    /// Check whether tmux is installed and reachable.
    pub fn is_available(&self) -> bool {
        Command::new("tmux")
            .arg("-V")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Launch a new detached tmux session that runs `claude` immediately.
    pub fn launch_claude_session(&self, name: &str, cwd: &str) -> Result<()> {
        let status = Command::new("tmux")
            .args(["-L", &self.socket_name])
            .args(["new-session", "-d", "-s", name, "-c", cwd, "claude"])
            .status()
            .wrap_err("failed to run tmux new-session")?;

        if !status.success() {
            bail!(
                "tmux new-session (claude) exited with status {} for session '{}'",
                status,
                name
            );
        }
        Ok(())
    }

    /// Resume (attach/switch-client) an existing session.
    ///
    /// If we're already inside tmux, uses `switch-client`; otherwise `attach`.
    pub fn resume_session(&self, name: &str) -> Result<()> {
        let inside_tmux = std::env::var("TMUX").is_ok();

        let status = if inside_tmux {
            Command::new("tmux")
                .args(["-L", &self.socket_name])
                .args(["switch-client", "-t", name])
                .status()
                .wrap_err("failed to run tmux switch-client")?
        } else {
            Command::new("tmux")
                .args(["-L", &self.socket_name])
                .args(["attach-session", "-t", name])
                .status()
                .wrap_err("failed to run tmux attach-session")?
        };

        if !status.success() {
            bail!(
                "tmux resume exited with status {} for session '{}'",
                status,
                name
            );
        }
        Ok(())
    }

    /// List sessions on the nexus socket.
    ///
    /// Parses the output of:
    /// ```text
    /// tmux -L nexus list-sessions -F '#{session_name}:#{window_name}:#{session_attached}:#{pane_current_command}'
    /// ```
    pub fn list_sessions(&self) -> Result<Vec<TmuxSessionInfo>> {
        let output = Command::new("tmux")
            .args(["-L", &self.socket_name])
            .args([
                "list-sessions",
                "-F",
                "#{session_name}:#{window_name}:#{session_attached}:#{pane_current_command}",
            ])
            .output()
            .wrap_err("failed to run tmux list-sessions")?;

        if !output.status.success() {
            // tmux exits non-zero when there are no sessions — that's fine.
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_list_sessions_output(&stdout))
    }

    /// Kill a session by name on the nexus socket.
    pub fn kill_session(&self, name: &str) -> Result<()> {
        let status = Command::new("tmux")
            .args(["-L", &self.socket_name])
            .args(["kill-session", "-t", name])
            .status()
            .wrap_err("failed to run tmux kill-session")?;

        if !status.success() {
            bail!(
                "tmux kill-session exited with status {} for '{}'",
                status,
                name
            );
        }
        Ok(())
    }

    /// Set up Nexus-specific key bindings on the nexus socket.
    ///
    /// Binds `Ctrl+Q` to detach from the session, providing a consistent
    /// way to return to the Nexus TUI.
    pub fn setup_keybindings(&self) -> Result<()> {
        let status = Command::new("tmux")
            .args(["-L", &self.socket_name])
            .args(["bind-key", "-n", "C-q", "detach-client"])
            .status()
            .wrap_err("failed to run tmux bind-key")?;

        if !status.success() {
            bail!("tmux bind-key exited with status {}", status);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Parse the raw output from `tmux list-sessions -F '...'` into structured
/// window info.
///
/// Expected format per line:
/// ```text
/// session_name:window_name:session_attached:pane_current_command
/// ```
///
/// - `session_attached > 0` -> `is_active = true`
/// - `pane_current_command` non-empty -> `Running`, otherwise `Idle`
fn parse_list_sessions_output(output: &str) -> Vec<TmuxSessionInfo> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(parse_session_line)
        .collect()
}

fn parse_session_line(line: &str) -> Option<TmuxSessionInfo> {
    // Split on ':', expecting at least 4 fields.
    // Session names and commands could theoretically contain colons,
    // so we split into at most 4 parts.
    let mut parts = line.splitn(4, ':');

    let session_name = parts.next()?.to_string();
    let window_name = parts.next()?.to_string();
    let attached_str = parts.next()?;
    let command = parts.next().unwrap_or("");

    let attached: u32 = attached_str.trim().parse().unwrap_or(0);
    let is_active = attached > 0;

    let status = if command.trim().is_empty() {
        TmuxSessionStatus::Idle
    } else {
        TmuxSessionStatus::Running
    };

    Some(TmuxSessionInfo {
        session_id: session_name,
        window_name,
        is_active,
        status,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_line() {
        let output = "my-session:bash:1:vim\n";
        let windows = parse_list_sessions_output(output);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].session_id, "my-session");
        assert_eq!(windows[0].window_name, "bash");
        assert!(windows[0].is_active);
        assert_eq!(windows[0].status, TmuxSessionStatus::Running);
    }

    #[test]
    fn test_parse_detached_idle() {
        let output = "nexus-001:zsh:0:\n";
        let windows = parse_list_sessions_output(output);
        assert_eq!(windows.len(), 1);
        assert!(!windows[0].is_active);
        assert_eq!(windows[0].status, TmuxSessionStatus::Idle);
    }

    #[test]
    fn test_parse_multiple_lines() {
        let output = "\
session-a:win1:1:claude\n\
session-b:win2:0:vim\n\
session-c:win3:0:\n";

        let windows = parse_list_sessions_output(output);
        assert_eq!(windows.len(), 3);

        assert_eq!(windows[0].session_id, "session-a");
        assert!(windows[0].is_active);
        assert_eq!(windows[0].status, TmuxSessionStatus::Running);

        assert_eq!(windows[1].session_id, "session-b");
        assert!(!windows[1].is_active);
        assert_eq!(windows[1].status, TmuxSessionStatus::Running);

        assert_eq!(windows[2].session_id, "session-c");
        assert!(!windows[2].is_active);
        assert_eq!(windows[2].status, TmuxSessionStatus::Idle);
    }

    #[test]
    fn test_parse_empty_output() {
        let windows = parse_list_sessions_output("");
        assert!(windows.is_empty());
    }

    #[test]
    fn test_parse_malformed_line_skipped() {
        let output = "only-one-part\nsession:win:0:cmd\n";
        let windows = parse_list_sessions_output(output);
        // First line has no colons -> skipped, second line is valid
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].session_id, "session");
    }

    #[test]
    fn test_parse_non_numeric_attached() {
        let output = "sess:win:abc:cmd\n";
        let windows = parse_list_sessions_output(output);
        assert_eq!(windows.len(), 1);
        // Non-numeric falls back to 0 -> not active
        assert!(!windows[0].is_active);
    }

    #[test]
    fn test_parse_command_with_colons() {
        // If the command itself contains colons, splitn(4, ':') keeps them
        let output = "sess:win:1:python:3.11:script.py\n";
        let windows = parse_list_sessions_output(output);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].session_id, "sess");
        assert_eq!(windows[0].window_name, "win");
        assert!(windows[0].is_active);
        assert_eq!(windows[0].status, TmuxSessionStatus::Running);
    }

    // -- Integration tests (require tmux installed) ----------------------

    #[test]
    #[ignore]
    fn test_is_available_with_tmux() {
        let mgr = TmuxManager::new("nexus-test");
        assert!(mgr.is_available());
    }

    #[test]
    #[ignore]
    fn test_launch_and_kill_session() {
        let mgr = TmuxManager::new("nexus-test-lk");

        // Launch claude session
        mgr.launch_claude_session("test-sess", "/tmp").unwrap();

        // Verify it appears in list
        let sessions = mgr.list_sessions().unwrap();
        assert!(
            sessions.iter().any(|s| s.session_id == "test-sess"),
            "session should appear in list"
        );

        // Kill
        mgr.kill_session("test-sess").unwrap();

        // Verify it's gone
        let sessions = mgr.list_sessions().unwrap();
        assert!(
            !sessions.iter().any(|s| s.session_id == "test-sess"),
            "session should be removed"
        );
    }

    #[test]
    #[ignore]
    fn test_setup_keybindings() {
        let mgr = TmuxManager::new("nexus-test-kb");

        // Need at least one session for bind-key to work
        mgr.launch_claude_session("kb-test", "/tmp").unwrap();
        mgr.setup_keybindings().unwrap();
        mgr.kill_session("kb-test").unwrap();
    }
}
