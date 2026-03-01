use std::collections::HashSet;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::DefaultTerminal;
use tachyonfx::Effect;

use crate::config::NexusConfig;
use crate::db::Database;
use crate::theme;
use crate::tmux::{sanitize_tmux_name, TmuxManager};
use crate::types::*;
use crate::ui;
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
    pub(crate) selection: SelectionState,
    pub(crate) tmux: TmuxManager,
    pub(crate) tmux_available: bool,
    pub(crate) tmux_sessions: Vec<TmuxSessionInfo>,
    last_tmux_poll: Instant,
    #[allow(dead_code)]
    config: NexusConfig,
    pub(crate) db: Database,
    // Cached values (updated on tmux poll and cursor change)
    pub(crate) cached_counts: (usize, usize),
    pub(crate) cached_selected: Option<SessionSummary>,
    // Status message overlay
    pub(crate) status_message: Option<(String, Instant)>,
    // Input state for CRUD operations
    pub(crate) input_mode: InputMode,
    pub(crate) input_buffer: String,
    pub(crate) input_context: Option<InputContext>,
    pub(crate) show_help: bool,
    pub(crate) show_dead_sessions: bool,
    // Group picker state
    pub(crate) picker_groups: Vec<(GroupId, String)>,
    pub(crate) picker_cursor: usize,
    // Set after returning from tmux attach to force a full redraw
    needs_full_redraw: bool,
}

impl App {
    pub fn new(
        config: NexusConfig,
        tree: Vec<TreeNode>,
        tmux: TmuxManager,
        tmux_available: bool,
        tmux_sessions: Vec<TmuxSessionInfo>,
        db: Database,
    ) -> Self {
        let tree_state = TreeState::new(&tree);
        let selection = SelectionState::default();
        let cached_counts = count_sessions(&tree);

        Self {
            should_quit: false,
            boot_done: false,
            last_tick: Instant::now(),
            boot_effects: theme::fx_boot(),
            tree,
            tree_state,
            selection,
            tmux,
            tmux_available,
            tmux_sessions,
            last_tmux_poll: Instant::now(),
            config,
            db,
            cached_counts,
            cached_selected: None,
            status_message: None,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            input_context: None,
            show_help: false,
            show_dead_sessions: false,
            picker_groups: Vec::new(),
            picker_cursor: 0,
            needs_full_redraw: false,
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_tick);
            self.last_tick = now;

            // Poll tmux for active sessions periodically
            if self.tmux_available && now.duration_since(self.last_tmux_poll) >= TMUX_POLL_INTERVAL
            {
                self.tmux_sessions = self.tmux.list_sessions().unwrap_or_default();
                self.reconcile_tmux_state();
                self.last_tmux_poll = now;
            }

            // Auto-clear status message after 5 seconds
            if let Some((_, ts)) = &self.status_message {
                if ts.elapsed() >= Duration::from_secs(5) {
                    self.status_message = None;
                }
            }

            if self.needs_full_redraw {
                terminal.clear()?;
                self.needs_full_redraw = false;
            }
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
        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::TextInput => self.handle_text_input_key(key),
            InputMode::Confirm => self.handle_confirm_key(key),
            InputMode::GroupPicker => self.handle_group_picker_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        // Help overlay intercepts everything except ? and Esc
        if self.show_help {
            match key.code {
                KeyCode::Char('?') | KeyCode::Esc => self.show_help = false,
                _ => self.show_help = false,
            }
            return;
        }

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
            KeyCode::Char('?') => {
                self.show_help = true;
                return;
            }
            KeyCode::Char('h') => {
                self.show_dead_sessions = !self.show_dead_sessions;
                self.refresh_tree();
                return;
            }
            _ => {}
        }

        self.handle_tree_key(key);
    }

    fn handle_tree_key(&mut self, key: KeyEvent) {
        match key.code {
            // CRUD keys
            KeyCode::Char('n') => self.start_new_session(),
            KeyCode::Char('G') => self.start_new_group(),
            KeyCode::Char('r') => self.start_rename(),
            KeyCode::Char('m') => self.start_move_session(),
            KeyCode::Char('d') => self.start_delete(),
            KeyCode::Char('x') => self.kill_tmux_session(),
            // Navigation / selection delegated to TreeState
            _ => {
                if let Some(action) = self.tree_state.handle_key(key, &self.tree) {
                    self.handle_tree_action(action);
                }
            }
        }
    }

    fn handle_tree_action(&mut self, action: TreeAction) {
        match action {
            TreeAction::Select(target) => {
                if let SelectionTarget::Session(ref id) = target {
                    let id_owned = id.clone();
                    self.selection.selected = Some(target);
                    self.refresh_cached_selected();
                    self.try_launch_session(&id_owned);
                } else {
                    self.selection.selected = Some(target);
                    self.refresh_cached_selected();
                }
            }
            TreeAction::ToggleExpand(_) => {}
            TreeAction::ScrollDown | TreeAction::ScrollUp => {
                if let Some(target) = self.tree_state.selected_target(&self.tree) {
                    self.selection.selected = Some(target);
                    self.refresh_cached_selected();
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Text input handling
    // -----------------------------------------------------------------------

    fn handle_text_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.input_context = None;
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Enter => {
                let buffer = self.input_buffer.clone();
                if buffer.trim().is_empty() {
                    return;
                }
                self.process_text_input(buffer);
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    fn process_text_input(&mut self, buffer: String) {
        let ctx = match self.input_context.take() {
            Some(c) => c,
            None => {
                self.input_mode = InputMode::Normal;
                return;
            }
        };

        match ctx {
            InputContext::NewSessionName => {
                // Move to CWD step, prefilled with current dir
                let default_cwd = std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.input_buffer = default_cwd;
                self.input_context = Some(InputContext::NewSessionCwd { name: buffer });
            }
            InputContext::NewSessionCwd { name } => {
                self.create_session(&name, &buffer);
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
            }
            InputContext::RenameSession { session_id } => {
                if let Err(e) = self.db.update_session_name(&session_id, &buffer) {
                    self.status_message =
                        Some((format!("rename failed: {e}"), Instant::now()));
                }
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.refresh_tree();
            }
            InputContext::RenameGroup { group_id } => {
                if let Err(e) = self.db.rename_group(group_id, &buffer) {
                    self.status_message =
                        Some((format!("rename failed: {e}"), Instant::now()));
                }
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.refresh_tree();
            }
            InputContext::NewGroupName => {
                match self.db.create_group(&buffer, "") {
                    Ok(_) => {}
                    Err(e) => {
                        self.status_message =
                            Some((format!("create group failed: {e}"), Instant::now()));
                    }
                }
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.refresh_tree();
            }
            _ => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Confirm handling
    // -----------------------------------------------------------------------

    fn handle_confirm_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let ctx = self.input_context.take();
                self.input_mode = InputMode::Normal;
                if let Some(ctx) = ctx {
                    self.process_confirm(ctx);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_context = None;
            }
            _ => {}
        }
    }

    fn process_confirm(&mut self, ctx: InputContext) {
        match ctx {
            InputContext::ConfirmDeleteSession { session_id, tmux_name } => {
                // Kill tmux if active
                if let Some(ref name) = tmux_name {
                    let _ = self.tmux.kill_session(name);
                }
                if let Err(e) = self.db.delete_session(&session_id) {
                    self.status_message =
                        Some((format!("delete failed: {e}"), Instant::now()));
                }
                self.refresh_tree();
            }
            InputContext::ConfirmDeleteGroup { group_id } => {
                if let Err(e) = self.db.delete_group(group_id) {
                    self.status_message =
                        Some((format!("delete failed: {e}"), Instant::now()));
                }
                self.refresh_tree();
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // Group picker handling
    // -----------------------------------------------------------------------

    fn handle_group_picker_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_context = None;
                self.picker_groups.clear();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.picker_groups.is_empty() {
                    self.picker_cursor = (self.picker_cursor + 1) % self.picker_groups.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.picker_groups.is_empty() {
                    if self.picker_cursor == 0 {
                        self.picker_cursor = self.picker_groups.len() - 1;
                    } else {
                        self.picker_cursor -= 1;
                    }
                }
            }
            KeyCode::Enter => {
                if let Some((gid, _)) = self.picker_groups.get(self.picker_cursor) {
                    let gid = *gid;
                    if let Some(InputContext::MoveSession { ref session_id }) = self.input_context {
                        let sid = session_id.clone();
                        if let Err(e) = self.db.move_session_to_group(&sid, gid) {
                            self.status_message =
                                Some((format!("move failed: {e}"), Instant::now()));
                        }
                    }
                }
                self.input_mode = InputMode::Normal;
                self.input_context = None;
                self.picker_groups.clear();
                self.refresh_tree();
            }
            _ => {}
        }
    }

    // -----------------------------------------------------------------------
    // CRUD action starters
    // -----------------------------------------------------------------------

    fn start_new_session(&mut self) {
        self.input_mode = InputMode::TextInput;
        self.input_context = Some(InputContext::NewSessionName);
        self.input_buffer.clear();
    }

    fn start_new_group(&mut self) {
        self.input_mode = InputMode::TextInput;
        self.input_context = Some(InputContext::NewGroupName);
        self.input_buffer.clear();
    }

    fn start_rename(&mut self) {
        let target = self.tree_state.selected_target(&self.tree);
        match target {
            Some(SelectionTarget::Session(id)) => {
                let current_name = find_session_in_tree(&self.tree, &id)
                    .map(|s| s.display_name.clone())
                    .unwrap_or_default();
                self.input_mode = InputMode::TextInput;
                self.input_context = Some(InputContext::RenameSession { session_id: id });
                self.input_buffer = current_name;
            }
            Some(SelectionTarget::Group(gid)) => {
                let current_name = find_group_in_tree(&self.tree, gid)
                    .map(|g| g.name.clone())
                    .unwrap_or_default();
                self.input_mode = InputMode::TextInput;
                self.input_context = Some(InputContext::RenameGroup { group_id: gid });
                self.input_buffer = current_name;
            }
            None => {}
        }
    }

    fn start_move_session(&mut self) {
        let target = self.tree_state.selected_target(&self.tree);
        if let Some(SelectionTarget::Session(id)) = target {
            match self.db.get_all_groups() {
                Ok(groups) if !groups.is_empty() => {
                    self.picker_groups = groups;
                    self.picker_cursor = 0;
                    self.input_mode = InputMode::GroupPicker;
                    self.input_context = Some(InputContext::MoveSession { session_id: id });
                }
                Ok(_) => {
                    self.status_message =
                        Some(("No groups available. Create one with G".to_string(), Instant::now()));
                }
                Err(e) => {
                    self.status_message =
                        Some((format!("failed to load groups: {e}"), Instant::now()));
                }
            }
        }
    }

    fn start_delete(&mut self) {
        let target = self.tree_state.selected_target(&self.tree);
        match target {
            Some(SelectionTarget::Session(id)) => {
                let tmux_name = find_session_in_tree(&self.tree, &id)
                    .and_then(|s| s.tmux_name.clone());
                self.input_mode = InputMode::Confirm;
                self.input_context = Some(InputContext::ConfirmDeleteSession {
                    session_id: id,
                    tmux_name,
                });
            }
            Some(SelectionTarget::Group(gid)) => {
                if gid == 0 {
                    return; // can't delete Ungrouped
                }
                self.input_mode = InputMode::Confirm;
                self.input_context = Some(InputContext::ConfirmDeleteGroup { group_id: gid });
            }
            None => {}
        }
    }

    fn kill_tmux_session(&mut self) {
        let target = self.tree_state.selected_target(&self.tree);
        if let Some(SelectionTarget::Session(id)) = target {
            if let Some(session) = find_session_in_tree(&self.tree, &id) {
                if session.status == SessionStatus::Active {
                    if let Some(ref tmux_name) = session.tmux_name {
                        if let Err(e) = self.tmux.kill_session(tmux_name) {
                            self.status_message =
                                Some((format!("kill failed: {e}"), Instant::now()));
                            return;
                        }
                    }
                    let _ = self.db.update_session_status(&id, SessionStatus::Detached);
                    self.refresh_tree();
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Session creation + launch
    // -----------------------------------------------------------------------

    fn create_session(&mut self, name: &str, cwd: &str) {
        let tmux_name = sanitize_tmux_name(name);
        match self.db.create_nexus_session(name, cwd, &tmux_name) {
            Ok(_id) => {
                if self.tmux_available {
                    if let Err(e) = self.tmux.launch_claude_session(&tmux_name, cwd) {
                        self.status_message =
                            Some((format!("tmux launch failed: {e}"), Instant::now()));
                        self.refresh_tree();
                        return;
                    }
                    let _ = self.tmux.setup_keybindings();
                    self.attach_tmux_session(&tmux_name);
                }
                self.refresh_tree();
            }
            Err(e) => {
                self.status_message =
                    Some((format!("create failed: {e}"), Instant::now()));
            }
        }
    }

    fn try_launch_session(&mut self, session_id: &str) {
        if !self.tmux_available {
            return;
        }

        let session = match find_session_in_tree(&self.tree, session_id) {
            Some(s) => s.clone(),
            None => return,
        };

        let cwd = match session.cwd.as_ref().map(|p| p.to_string_lossy().to_string()) {
            Some(c) => c,
            None => return,
        };

        let tmux_name = match session.tmux_name.as_ref() {
            Some(n) => n.clone(),
            None => sanitize_tmux_name(session_id),
        };

        match session.status {
            SessionStatus::Active => {
                // Attach to running session
                self.attach_tmux_session(&tmux_name);
            }
            SessionStatus::Detached => {
                // Re-launch claude in same cwd
                if let Err(e) = self.tmux.launch_claude_session(&tmux_name, &cwd) {
                    self.status_message =
                        Some((format!("tmux launch failed: {e}"), Instant::now()));
                    return;
                }
                let _ = self.tmux.setup_keybindings();
                let _ = self.db.update_session_status(session_id, SessionStatus::Active);
                self.attach_tmux_session(&tmux_name);
                self.refresh_tree();
            }
            SessionStatus::Dead => {
                self.status_message = Some((
                    "Dead session (not resumable). Only Nexus-created sessions can be resumed."
                        .to_string(),
                    Instant::now(),
                ));
            }
        }
    }

    /// Suspend the TUI, attach to a tmux session, then restore the TUI.
    fn attach_tmux_session(&mut self, tmux_name: &str) {
        // Leave ratatui's alternate screen and raw mode so tmux can take over
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);

        let result = self.tmux.resume_session(tmux_name);

        // Restore ratatui's terminal state
        let _ = crossterm::execute!(std::io::stdout(), EnterAlternateScreen);
        let _ = crossterm::terminal::enable_raw_mode();

        // Force a full redraw — ratatui's internal buffer is stale after tmux
        self.needs_full_redraw = true;

        if let Err(e) = result {
            self.status_message =
                Some((format!("tmux attach failed: {e}"), Instant::now()));
        }
    }

    // -----------------------------------------------------------------------
    // tmux reconciliation
    // -----------------------------------------------------------------------

    fn reconcile_tmux_state(&mut self) {
        let active_names: HashSet<&str> = self
            .tmux_sessions
            .iter()
            .map(|s| s.session_id.as_str())
            .collect();

        let mut changed = false;
        reconcile_recursive(&mut self.tree, &active_names, &self.db, &mut changed);

        if changed {
            self.tree_state.invalidate_cache();
        }

        self.cached_counts = count_sessions(&self.tree);
    }

    // -----------------------------------------------------------------------
    // Tree refresh
    // -----------------------------------------------------------------------

    pub(crate) fn refresh_tree(&mut self) {
        if let Ok(tree) = self.db.get_visible_tree(self.show_dead_sessions) {
            self.tree = tree;
            self.tree_state.invalidate_cache();
            self.cached_counts = count_sessions(&self.tree);
            self.refresh_cached_selected();
        }
    }

    fn refresh_cached_selected(&mut self) {
        self.cached_selected = match self.selection.selected.as_ref() {
            Some(SelectionTarget::Session(id)) => {
                find_session_in_tree(&self.tree, id).cloned()
            }
            _ => None,
        };
    }

    pub(crate) fn selected_session(&self) -> Option<&SessionSummary> {
        self.cached_selected.as_ref()
    }

    pub(crate) fn session_counts(&self) -> (usize, usize) {
        self.cached_counts
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

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

fn find_group_in_tree(tree: &[TreeNode], group_id: GroupId) -> Option<&GroupNode> {
    for node in tree {
        if let TreeNode::Group(g) = node {
            if g.id == group_id {
                return Some(g);
            }
            if let Some(found) = find_group_in_tree(&g.children, group_id) {
                return Some(found);
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

fn reconcile_recursive(
    tree: &mut [TreeNode],
    active_names: &HashSet<&str>,
    db: &Database,
    changed: &mut bool,
) {
    for node in tree.iter_mut() {
        match node {
            TreeNode::Session(s) => {
                let tmux_name = s.tmux_name.as_deref().unwrap_or("");
                let is_in_tmux = !tmux_name.is_empty() && active_names.contains(tmux_name);

                if is_in_tmux && s.status != SessionStatus::Active {
                    s.status = SessionStatus::Active;
                    s.is_active = true;
                    let _ = db.update_session_status(&s.session_id, SessionStatus::Active);
                    *changed = true;
                } else if !is_in_tmux && s.status == SessionStatus::Active {
                    s.status = SessionStatus::Detached;
                    s.is_active = false;
                    let _ = db.update_session_status(&s.session_id, SessionStatus::Detached);
                    *changed = true;
                } else {
                    s.is_active = is_in_tmux;
                }
            }
            TreeNode::Group(g) => {
                reconcile_recursive(&mut g.children, active_names, db, changed);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
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
        assert_eq!(sanitized, id);
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
    fn test_find_session_in_tree() {
        use crate::mock;
        let tree = mock::mock_tree();

        let found = find_session_in_tree(&tree, "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
        assert!(found.is_some());
        assert_eq!(found.unwrap().display_name, "feat/scanner");

        let not_found = find_session_in_tree(&tree, "nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_find_group_in_tree() {
        use crate::mock;
        let tree = mock::mock_tree();

        let found = find_group_in_tree(&tree, 1);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "nexus");

        let not_found = find_group_in_tree(&tree, 999);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_reconcile_marks_detached() {
        use crate::mock;
        let mut tree = mock::mock_tree();
        let db = Database::open_in_memory().unwrap();
        let active_names: HashSet<&str> = HashSet::new(); // no tmux sessions
        let mut changed = false;

        reconcile_recursive(&mut tree, &active_names, &db, &mut changed);

        // Sessions that were Active should now be Detached
        let s = find_session_in_tree(&tree, "a1b2c3d4-e5f6-7890-abcd-ef1234567890").unwrap();
        assert_eq!(s.status, SessionStatus::Detached);
        assert!(!s.is_active);
        assert!(changed);
    }
}
