use std::collections::HashSet;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::DefaultTerminal;
use tachyonfx::Effect;

use crate::config::NexusConfig;
use crate::theme;
use crate::tmux::TmuxManager;
use crate::types::*;
use crate::ui;
use crate::widgets::radar_state::RadarState;
use crate::widgets::tree_state::{TreeAction, TreeState};

const TICK_RATE: Duration = Duration::from_millis(16);
const TMUX_POLL_INTERVAL: Duration = Duration::from_secs(2);

pub struct App {
    pub should_quit: bool,
    pub(crate) boot_done: bool,
    last_tick: Instant,
    pub(crate) boot_effects: Vec<Effect>,
    pub(crate) tree: Vec<TreeNode>,
    pub(crate) tree_state: TreeState,
    pub(crate) radar_state: RadarState,
    pub(crate) selection: SelectionState,
    pub(crate) tmux: TmuxManager,
    pub(crate) tmux_available: bool,
    pub(crate) tmux_windows: Vec<TmuxWindowInfo>,
    last_tmux_poll: Instant,
    #[allow(dead_code)]
    config: NexusConfig,
    // Cached values (updated on tmux poll and cursor change) — Todo 015
    pub(crate) cached_counts: (usize, usize),
    pub(crate) cached_selected: Option<SessionSummary>,
    // Status message overlay — Todo 023
    pub(crate) status_message: Option<(String, Instant)>,
}

impl App {
    pub fn new(
        config: NexusConfig,
        tree: Vec<TreeNode>,
        tmux: TmuxManager,
        tmux_available: bool,
        tmux_windows: Vec<TmuxWindowInfo>,
    ) -> Self {
        let tree_state = TreeState::new(&tree);
        let mut radar_state = RadarState::new();
        radar_state.compute_blips(&tree);
        let selection = SelectionState::default();
        let cached_counts = count_sessions(&tree);

        Self {
            should_quit: false,
            boot_done: false,
            last_tick: Instant::now(),
            boot_effects: theme::fx_boot(),
            tree,
            tree_state,
            radar_state,
            selection,
            tmux,
            tmux_available,
            tmux_windows,
            last_tmux_poll: Instant::now(),
            config,
            cached_counts,
            cached_selected: None,
            status_message: None,
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_tick);
            self.last_tick = now;

            // Advance radar sweep
            self.radar_state.advance_sweep(elapsed.as_secs_f64());

            // Poll tmux for active windows periodically
            if self.tmux_available && now.duration_since(self.last_tmux_poll) >= TMUX_POLL_INTERVAL
            {
                self.tmux_windows = self.tmux.list_windows().unwrap_or_default();

                // Mark sessions as active based on tmux windows
                mark_active_sessions(&mut self.tree, &self.tmux_windows);

                // Invalidate tree cache since tree data changed
                self.tree_state.invalidate_cache();

                // Refresh cached counts
                self.cached_counts = count_sessions(&self.tree);

                self.last_tmux_poll = now;
            }

            // Auto-clear status message after 5 seconds
            if let Some((_, ts)) = &self.status_message {
                if ts.elapsed() >= Duration::from_secs(5) {
                    self.status_message = None;
                }
            }

            // Always redraw: radar sweep animates continuously
            terminal.draw(|frame| ui::draw(frame, &mut self, elapsed))?;

            let poll_timeout = if self.boot_done {
                TICK_RATE
            } else {
                TICK_RATE.saturating_sub(now.elapsed())
            };

            if event::poll(poll_timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key);
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        // Global keys first
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('c')
                if key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
            {
                self.should_quit = true;
                return;
            }
            KeyCode::Tab => {
                self.selection.focused_panel = match self.selection.focused_panel {
                    FocusPanel::Tree => FocusPanel::Radar,
                    FocusPanel::Radar => FocusPanel::Tree,
                };
                return;
            }
            _ => {}
        }

        // Panel-specific keys
        match self.selection.focused_panel {
            FocusPanel::Tree => {
                if let Some(action) = self.tree_state.handle_key(key, &self.tree) {
                    self.handle_tree_action(action);
                }
            }
            FocusPanel::Radar => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.radar_state.move_cursor_next();
                    if let Some(id) = self.radar_state.selected_session() {
                        self.selection.selected =
                            Some(SelectionTarget::Session(id.to_string()));
                        self.refresh_cached_selected();
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.radar_state.move_cursor_prev();
                    if let Some(id) = self.radar_state.selected_session() {
                        self.selection.selected =
                            Some(SelectionTarget::Session(id.to_string()));
                        self.refresh_cached_selected();
                    }
                }
                KeyCode::Enter => {
                    // Select on radar, then try to launch/resume tmux
                    if let Some(id) = self.radar_state.selected_session().map(str::to_string) {
                        self.selection.selected =
                            Some(SelectionTarget::Session(id.clone()));
                        self.refresh_cached_selected();
                        self.try_launch_session(&id);
                    }
                }
                _ => {}
            },
        }
    }

    fn handle_tree_action(&mut self, action: TreeAction) {
        match action {
            TreeAction::Select(target) => {
                if let SelectionTarget::Session(ref id) = target {
                    self.radar_state.select_by_session_id(id);
                    let id_owned = id.clone();
                    self.selection.selected = Some(target);
                    self.refresh_cached_selected();
                    self.try_launch_session(&id_owned);
                } else {
                    self.selection.selected = Some(target);
                    self.refresh_cached_selected();
                }
            }
            TreeAction::ToggleExpand(_) => {
                self.radar_state.compute_blips(&self.tree);
            }
            TreeAction::ScrollDown | TreeAction::ScrollUp => {
                if let Some(target) = self.tree_state.selected_target(&self.tree) {
                    if let SelectionTarget::Session(ref id) = target {
                        self.radar_state.select_by_session_id(id);
                    }
                    self.selection.selected = Some(target);
                    self.refresh_cached_selected();
                }
            }
        }
    }

    fn try_launch_session(&mut self, session_id: &str) {
        if !self.tmux_available {
            return;
        }

        // Find the session's cwd (Todo 031: use find_session_in_tree directly)
        let cwd = match find_session_in_tree(&self.tree, session_id)
            .and_then(|s| s.cwd.as_ref().map(|p| p.to_string_lossy().to_string()))
        {
            Some(c) => c,
            None => return,
        };

        // Sanitize name for tmux (Todo 013: keep full ID, replace non-alnum with dash)
        let tmux_name = sanitize_tmux_name(session_id);

        // Check if already running -- compare against sanitized name (Todo 013)
        let already_running = self
            .tmux_windows
            .iter()
            .any(|w| w.session_id == tmux_name);

        // Surface tmux errors instead of silently swallowing (Todo 023)
        if already_running {
            if let Err(e) = self.tmux.resume_session(&tmux_name) {
                self.status_message = Some((format!("tmux resume failed: {e}"), Instant::now()));
            }
        } else if let Err(e) = self.tmux.launch_session(&tmux_name, &cwd) {
            self.status_message = Some((format!("tmux launch failed: {e}"), Instant::now()));
        }
    }

    /// Refresh the cached selected session from current selection state.
    fn refresh_cached_selected(&mut self) {
        self.cached_selected = match self.selection.selected.as_ref() {
            Some(SelectionTarget::Session(id)) => {
                find_session_in_tree(&self.tree, id).cloned()
            }
            _ => None,
        };
    }

    /// Get the currently selected session, if any. Uses cached value (Todo 015).
    pub(crate) fn selected_session(&self) -> Option<&SessionSummary> {
        self.cached_selected.as_ref()
    }

    /// Count total and active sessions in the tree. Uses cached value (Todo 015).
    pub(crate) fn session_counts(&self) -> (usize, usize) {
        self.cached_counts
    }
}

fn find_session_in_tree<'a>(tree: &'a [TreeNode], session_id: &str) -> Option<&'a SessionSummary> {
    for node in tree {
        match node {
            TreeNode::Session(s) => {
                if s.session_id == session_id {
                    return Some(s);
                }
            }
            TreeNode::Group(g) => {
                if let Some(s) = find_session_in_tree(&g.children, session_id) {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn count_sessions(tree: &[TreeNode]) -> (usize, usize) {
    let mut total = 0;
    let mut active = 0;
    for node in tree {
        match node {
            TreeNode::Session(s) => {
                total += 1;
                if s.is_active {
                    active += 1;
                }
            }
            TreeNode::Group(g) => {
                let (t, a) = count_sessions(&g.children);
                total += t;
                active += a;
            }
        }
    }
    (total, active)
}

/// Mark sessions as active based on tmux windows. Uses a HashSet for O(S+W) (Todo 018).
fn mark_active_sessions(tree: &mut [TreeNode], windows: &[TmuxWindowInfo]) {
    let active_ids: HashSet<&str> = windows.iter().map(|w| w.session_id.as_str()).collect();
    mark_active_recursive(tree, &active_ids);
}

fn mark_active_recursive(tree: &mut [TreeNode], active_ids: &HashSet<&str>) {
    for node in tree.iter_mut() {
        match node {
            TreeNode::Session(s) => {
                s.is_active = active_ids.contains(s.session_id.as_str());
            }
            TreeNode::Group(g) => {
                mark_active_recursive(&mut g.children, active_ids);
            }
        }
    }
}

/// Sanitize a string for use as a tmux session name.
/// Replaces non-alphanumeric characters (except dash) with dashes.
/// Does NOT truncate -- keeps full ID for reliable matching (Todo 013).
fn sanitize_tmux_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests (Todo 028)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_tmux_name_ascii() {
        assert_eq!(sanitize_tmux_name("hello-world"), "hello-world");
        assert_eq!(sanitize_tmux_name("a1b2c3"), "a1b2c3");
    }

    #[test]
    fn test_sanitize_tmux_name_special_chars() {
        assert_eq!(sanitize_tmux_name("foo.bar/baz"), "foo-bar-baz");
        assert_eq!(sanitize_tmux_name("a b c"), "a-b-c");
    }

    #[test]
    fn test_sanitize_tmux_name_preserves_full_id() {
        let id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
        let sanitized = sanitize_tmux_name(id);
        assert_eq!(sanitized, id); // UUIDs should pass through unchanged
    }

    #[test]
    fn test_count_sessions_empty() {
        let tree: Vec<TreeNode> = vec![];
        assert_eq!(count_sessions(&tree), (0, 0));
    }

    #[test]
    fn test_count_sessions_nested() {
        use crate::mock;
        let tree = mock::mock_tree();
        let (total, active) = count_sessions(&tree);
        assert_eq!(total, 5);
        assert_eq!(active, 2);
    }

    #[test]
    fn test_mark_active_sessions_matching() {
        use crate::mock;
        let mut tree = mock::mock_tree();

        // First, clear all active flags
        clear_active(&mut tree);

        let windows = vec![TmuxWindowInfo {
            session_id: "a1b2c3d4-e5f6-7890-abcd-ef1234567890".to_string(),
            window_name: "test".to_string(),
            is_active: true,
            status: TmuxSessionStatus::Running,
        }];

        mark_active_sessions(&mut tree, &windows);

        let (_, active) = count_sessions(&tree);
        assert_eq!(active, 1);
    }

    #[test]
    fn test_mark_active_sessions_no_match() {
        use crate::mock;
        let mut tree = mock::mock_tree();
        clear_active(&mut tree);

        let windows = vec![TmuxWindowInfo {
            session_id: "nonexistent-id".to_string(),
            window_name: "test".to_string(),
            is_active: true,
            status: TmuxSessionStatus::Running,
        }];

        mark_active_sessions(&mut tree, &windows);

        let (_, active) = count_sessions(&tree);
        assert_eq!(active, 0);
    }

    #[test]
    fn test_find_session_in_tree() {
        use crate::mock;
        let tree = mock::mock_tree();

        let found = find_session_in_tree(&tree, "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
        assert!(found.is_some());
        assert_eq!(found.unwrap().display_name, "feat/scanner");

        let not_found = find_session_in_tree(&tree, "nonexistent");
        assert!(not_found.is_none());
    }

    fn clear_active(tree: &mut [TreeNode]) {
        for node in tree {
            match node {
                TreeNode::Session(s) => s.is_active = false,
                TreeNode::Group(g) => clear_active(&mut g.children),
            }
        }
    }
}
