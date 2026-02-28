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
    pub(crate) dirty: bool,
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

        Self {
            should_quit: false,
            dirty: true,
            boot_done: false,
            last_tick: Instant::now(),
            boot_effects: theme::create_boot_effects(),
            tree,
            tree_state,
            radar_state,
            selection,
            tmux,
            tmux_available,
            tmux_windows,
            last_tmux_poll: Instant::now(),
            config,
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

                self.last_tmux_poll = now;
                self.dirty = true;
            }

            // Always redraw: radar sweep animates continuously
            terminal.draw(|frame| ui::draw(frame, &mut self, elapsed))?;
            self.dirty = false;

            let poll_timeout = if self.boot_done {
                TICK_RATE
            } else {
                TICK_RATE.saturating_sub(now.elapsed())
            };

            if event::poll(poll_timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.dirty = true;
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
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.radar_state.move_cursor_prev();
                    if let Some(id) = self.radar_state.selected_session() {
                        self.selection.selected =
                            Some(SelectionTarget::Session(id.to_string()));
                    }
                }
                KeyCode::Enter => {
                    // Select on radar, then try to launch/resume tmux
                    if let Some(id) = self.radar_state.selected_session().map(str::to_string) {
                        self.selection.selected =
                            Some(SelectionTarget::Session(id.clone()));
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
                    self.try_launch_session(&id_owned);
                } else {
                    self.selection.selected = Some(target);
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
                }
            }
            _ => {}
        }
    }

    fn try_launch_session(&mut self, session_id: &str) {
        if !self.tmux_available {
            return;
        }

        // Find the session's cwd
        let cwd = find_session_cwd(&self.tree, session_id);
        let cwd = match cwd {
            Some(c) => c,
            None => return,
        };

        // Sanitize name for tmux (replace non-alphanumeric with dash)
        let tmux_name = sanitize_tmux_name(session_id);

        // Check if already running
        let already_running = self
            .tmux_windows
            .iter()
            .any(|w| w.session_id == session_id);

        if already_running {
            let _ = self.tmux.resume_session(&tmux_name);
        } else {
            let _ = self.tmux.launch_session(&tmux_name, &cwd);
        }
    }

    /// Get the currently selected session, if any.
    pub(crate) fn selected_session(&self) -> Option<&SessionSummary> {
        let target = self.selection.selected.as_ref()?;
        match target {
            SelectionTarget::Session(id) => find_session_in_tree(&self.tree, id),
            SelectionTarget::Group(_) => None,
        }
    }

    /// Count total and active sessions in the tree.
    pub(crate) fn session_counts(&self) -> (usize, usize) {
        count_sessions(&self.tree)
    }
}

fn find_session_cwd(tree: &[TreeNode], session_id: &str) -> Option<String> {
    for node in tree {
        match node {
            TreeNode::Session(s) => {
                if s.session_id == session_id {
                    return s.cwd.as_ref().map(|p| p.to_string_lossy().to_string());
                }
            }
            TreeNode::Group(g) => {
                if let Some(cwd) = find_session_cwd(&g.children, session_id) {
                    return Some(cwd);
                }
            }
        }
    }
    None
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

fn mark_active_sessions(tree: &mut [TreeNode], windows: &[TmuxWindowInfo]) {
    for node in tree.iter_mut() {
        match node {
            TreeNode::Session(s) => {
                s.is_active = windows.iter().any(|w| w.session_id == s.session_id);
            }
            TreeNode::Group(g) => {
                mark_active_sessions(&mut g.children, windows);
            }
        }
    }
}

fn sanitize_tmux_name(s: &str) -> String {
    // Use last 8 chars of session ID as tmux name
    let short = if s.len() > 8 { &s[s.len() - 8..] } else { s };
    short
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}
