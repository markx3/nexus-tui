//! Background scanner for Claude Code halt-state detection.
//!
//! Polls all live tmux sessions every ~2 seconds using lightweight text-only
//! capture (`capture_pane_tail`). Pattern-matches the last 20 lines for known
//! halt signatures (permission prompts, MCP confirmations) and sends the set
//! of halted session names to the main thread via mpsc channel.

use std::collections::HashSet;
use std::sync::mpsc;
use std::time::Duration;

use crate::tmux::TmuxManager;

const SCAN_INTERVAL: Duration = Duration::from_secs(2);
const CAPTURE_LINES: u32 = 20;

/// Known halt patterns — substring matches against each captured line.
///
/// These match Claude Code's distinctive permission/confirmation prompts.
/// Kept simple (no regex) for speed and clarity.
const HALT_PATTERNS: &[&str] = &[
    // Tool permission prompts: "Allow? (Y)es | (N)o | (A)lways"
    "(Y)es",
    // MCP / destructive action confirmations
    "Do you want to proceed?",
    // AskUserQuestion interactive selection prompts
    "Enter to select",
];

/// Spawn the feedback scanner thread.
///
/// Returns a receiver that yields `HashSet<String>` of tmux session names
/// currently in a halt state. Only sends when the set changes.
pub fn spawn(tmux: TmuxManager) -> mpsc::Receiver<HashSet<String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("nexus-feedback".to_string())
        .spawn(move || scanner_loop(tmux, tx))
        .expect("failed to spawn feedback scanner thread");
    rx
}

fn scanner_loop(tmux: TmuxManager, tx: mpsc::Sender<HashSet<String>>) {
    let mut last_set: HashSet<String> = HashSet::new();

    loop {
        let mut halted = HashSet::new();

        if let Ok(sessions) = tmux.list_sessions() {
            for session in &sessions {
                if let Ok(text) = tmux.capture_pane_tail(&session.session_id, CAPTURE_LINES) {
                    if has_halt_pattern(&text) {
                        halted.insert(session.session_id.clone());
                    }
                }
            }
        }

        // Only send if the set changed
        if halted != last_set {
            last_set.clone_from(&halted);
            if tx.send(halted).is_err() {
                return; // Main thread dropped receiver
            }
        }

        std::thread::sleep(SCAN_INTERVAL);
    }
}

/// Check if any line in the captured text contains a known halt pattern.
fn has_halt_pattern(text: &str) -> bool {
    text.lines()
        .any(|line| HALT_PATTERNS.iter().any(|p| line.contains(p)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_permission_prompt() {
        let text = "  ○ Read src/main.rs\n\n  Allow? (Y)es | (N)o | (A)lways\n";
        assert!(has_halt_pattern(text));
    }

    #[test]
    fn detects_yes_no_always_line() {
        let text = "Some output\n  (Y)es | (N)o | (A)lways allow\n";
        assert!(has_halt_pattern(text));
    }

    #[test]
    fn detects_proceed_prompt() {
        let text = "Warning: this will delete files\nDo you want to proceed? (y/n)\n";
        assert!(has_halt_pattern(text));
    }

    #[test]
    fn ignores_normal_output() {
        let text = "Building project...\n✓ All tests passed\n> ";
        assert!(!has_halt_pattern(text));
    }

    #[test]
    fn ignores_empty() {
        assert!(!has_halt_pattern(""));
        assert!(!has_halt_pattern("\n\n\n"));
    }

    #[test]
    fn ignores_claude_idle_prompt() {
        // The normal "ready for next message" state should NOT trigger
        let text = "\n> \n";
        assert!(!has_halt_pattern(text));
    }

    #[test]
    fn detects_ask_user_question() {
        let text = "Which option?\n\n❯ 1. Option A\n  2. Option B\n\nEnter to select · ↑/↓ to navigate · Esc to cancel\n";
        assert!(has_halt_pattern(text));
    }

    #[test]
    fn detects_with_surrounding_content() {
        let text = "line 1\nline 2\nline 3\nline 4\nline 5\n\
                    line 6\nline 7\nline 8\n\
                    Allow? (Y)es | (N)o | (A)lways\nline 10\n";
        assert!(has_halt_pattern(text));
    }
}
