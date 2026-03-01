use std::collections::HashSet;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::event::{EnableBracketedPaste, DisableBracketedPaste};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::DefaultTerminal;
use tachyonfx::Effect;

use crate::capture_worker;
use crate::config::NexusConfig;
use crate::db::Database;
use crate::theme;
use crate::tmux::{sanitize_tmux_name, TmuxManager};
use crate::types::*;
use crate::ui;
use crate::widgets::interactor_state::InteractorState;
use crate::widgets::tree_state::{TreeAction, TreeState};

const TICK_RATE: Duration = Duration::from_millis(16);
const TMUX_POLL_INTERVAL: Duration = Duration::from_secs(2);
const LOGO_FRAME_INTERVAL: Duration = Duration::from_millis(300);

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
    // Path completion state (active during NewSessionCwd input)
    pub(crate) path_suggestions: Vec<String>,
    pub(crate) path_suggestion_cursor: usize,
    // Set after returning from tmux attach to force a full redraw
    needs_full_redraw: bool,
    // Session interactor state (None if tmux unavailable)
    pub(crate) interactor_state: Option<InteractorState>,
    // Logo animation state
    pub(crate) logo_frame: usize,
    logo_last_advance: Instant,
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

        // Spawn capture worker and configure tmux server if tmux is available
        let interactor_state = if tmux_available {
            // Configure true color + keybindings (no-op if server not yet started)
            if !tmux_sessions.is_empty() {
                let _ = tmux.configure_server();
            }
            let (session_tx, content_rx, nudge_tx) = capture_worker::spawn(tmux.clone());
            Some(InteractorState::new(tmux.clone(), content_rx, session_tx, nudge_tx))
        } else {
            None
        };

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
            path_suggestions: Vec::new(),
            path_suggestion_cursor: 0,
            needs_full_redraw: false,
            interactor_state,
            logo_frame: 0,
            logo_last_advance: Instant::now(),
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // Enable bracketed paste so crossterm emits Event::Paste
        let _ = crossterm::execute!(std::io::stdout(), EnableBracketedPaste);

        let result = self.event_loop(&mut terminal);

        let _ = crossterm::execute!(std::io::stdout(), DisableBracketedPaste);
        result
    }

    fn event_loop(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_tick);
            self.last_tick = now;

            // Poll capture worker for new content
            if let Some(ref mut is) = self.interactor_state {
                is.poll_content();
            }

            // Poll tmux for active sessions periodically
            if self.tmux_available && now.duration_since(self.last_tmux_poll) >= TMUX_POLL_INTERVAL
            {
                self.tmux_sessions = self.tmux.list_sessions().unwrap_or_default();
                self.reconcile_tmux_state();
                self.last_tmux_poll = now;
            }

            // Advance logo animation frame
            if now.duration_since(self.logo_last_advance) >= LOGO_FRAME_INTERVAL {
                self.logo_frame = self.logo_frame.wrapping_add(1);
                self.logo_last_advance = now;
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
            terminal.draw(|frame| ui::draw(frame, self, elapsed))?;

            let poll_timeout = if self.boot_done {
                TICK_RATE
            } else {
                TICK_RATE.saturating_sub(now.elapsed())
            };

            if event::poll(poll_timeout)? {
                let ev = event::read()?;
                self.handle_event(ev);
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, event: Event) {
        // Modal overlays intercept key events directly (not forwarded to tmux)
        if self.input_mode != InputMode::Normal {
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    match self.input_mode {
                        InputMode::TextInput => self.handle_text_input_key(key),
                        InputMode::Confirm => self.handle_confirm_key(key),
                        InputMode::GroupPicker => self.handle_group_picker_key(key),
                        InputMode::Normal => unreachable!(),
                    }
                }
            }
            return;
        }

        // Help overlay — any key dismisses
        if self.show_help {
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    self.show_help = false;
                }
            }
            return;
        }

        // Resize events — update interactor pane geometry
        if let Event::Resize(cols, rows) = event {
            if let Some(ref mut is) = self.interactor_state {
                let (ic, ir) = interactor_inner_size(cols, rows);
                is.resize_if_needed(ic, ir);
            }
            return;
        }

        // Get the current tmux name for forwarding
        let current_tmux_name = self.cached_selected.as_ref()
            .filter(|s| s.status == SessionStatus::Active)
            .and_then(|s| s.tmux_name.clone());

        // Delegate to interactor's route_event
        if let Some(ref mut is) = self.interactor_state {
            let result = is.route_event(&event, current_tmux_name.as_deref());
            match result {
                RouteResult::Forwarded => return,
                RouteResult::NexusCommand(cmd) => {
                    self.dispatch_nexus_command(cmd);
                    return;
                }
                RouteResult::Ignored => {}
            }
        }

        // Fallback for when interactor_state is None (tmux unavailable):
        // handle keys directly for tree navigation and CRUD
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                self.handle_fallback_key(key);
            }
        }
    }

    fn dispatch_nexus_command(&mut self, cmd: NexusCommand) {
        match cmd {
            NexusCommand::CursorDown => {
                if let Some(action) = self.tree_state.handle_key(
                    KeyEvent::new(KeyCode::Down, crossterm::event::KeyModifiers::NONE),
                    &self.tree,
                ) {
                    self.handle_tree_action(action);
                }
            }
            NexusCommand::CursorUp => {
                if let Some(action) = self.tree_state.handle_key(
                    KeyEvent::new(KeyCode::Up, crossterm::event::KeyModifiers::NONE),
                    &self.tree,
                ) {
                    self.handle_tree_action(action);
                }
            }
            NexusCommand::ToggleExpand => {
                if let Some(action) = self.tree_state.handle_key(
                    KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE),
                    &self.tree,
                ) {
                    self.handle_tree_action(action);
                }
            }
            NexusCommand::NewSession => self.start_new_session(),
            NexusCommand::DeleteSelected => self.start_delete(),
            NexusCommand::RenameSelected => self.start_rename(),
            NexusCommand::MoveSession => self.start_move_session(),
            NexusCommand::NewGroup => self.start_new_group(),
            NexusCommand::KillTmux => self.kill_tmux_session(),
            NexusCommand::FullscreenAttach => self.fullscreen_attach(),
            NexusCommand::ToggleHelp => {
                self.show_help = !self.show_help;
            }
            NexusCommand::Quit => {
                self.should_quit = true;
            }
            NexusCommand::ToggleDeadSessions => {
                self.show_dead_sessions = !self.show_dead_sessions;
                self.refresh_tree();
            }
        }
    }

    /// Fullscreen attach: suspend nexus TUI and attach to the selected tmux session.
    fn fullscreen_attach(&mut self) {
        let tmux_name = match self.cached_selected.as_ref() {
            Some(s) if s.status == SessionStatus::Active => s.tmux_name.clone(),
            _ => {
                self.status_message = Some((
                    "No active session to attach".to_string(),
                    Instant::now(),
                ));
                return;
            }
        };
        if let Some(name) = tmux_name {
            self.attach_tmux_session(&name);
        }
    }

    /// Fallback key handler when interactor_state is None (tmux unavailable).
    fn handle_fallback_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('h') => {
                self.show_dead_sessions = !self.show_dead_sessions;
                self.refresh_tree();
            }
            KeyCode::Char('n') => self.start_new_session(),
            KeyCode::Char('G') => self.start_new_group(),
            KeyCode::Char('r') => self.start_rename(),
            KeyCode::Char('m') => self.start_move_session(),
            KeyCode::Char('d') => self.start_delete(),
            KeyCode::Char('x') => self.kill_tmux_session(),
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
                if matches!(target, SelectionTarget::Session(_)) {
                    self.selection.selected = Some(target);
                    self.refresh_cached_selected();
                    self.sync_interactor_to_selection();
                    self.ensure_session_launched();
                } else {
                    self.selection.selected = Some(target);
                    self.refresh_cached_selected();
                    // Group node selected — clear interactor
                    if let Some(ref mut is) = self.interactor_state {
                        is.clear();
                    }
                }
            }
            TreeAction::ToggleExpand(_) => {}
            TreeAction::ScrollDown | TreeAction::ScrollUp => {
                if let Some(target) = self.tree_state.selected_target(&self.tree) {
                    self.selection.selected = Some(target);
                    self.refresh_cached_selected();
                    self.sync_interactor_to_selection();
                }
            }
        }
    }

    /// Sync interactor state to the currently selected session.
    ///
    /// Also triggers an immediate tmux pane resize so the embedded session
    /// renders at the correct dimensions before the first capture arrives.
    fn sync_interactor_to_selection(&mut self) {
        if let Some(ref session) = self.cached_selected {
            if let Some(ref mut is) = self.interactor_state {
                is.switch_session(session);
                // Resize the tmux pane to match the interactor panel dimensions.
                if let Ok((cols, rows)) = crossterm::terminal::size() {
                    let (ic, ir) = interactor_inner_size(cols, rows);
                    is.resize_if_needed(ic, ir);
                }
            }
        } else if let Some(ref mut is) = self.interactor_state {
            is.clear();
        }
    }

    // -----------------------------------------------------------------------
    // Text input handling
    // -----------------------------------------------------------------------

    fn handle_text_input_key(&mut self, key: KeyEvent) {
        let is_cwd = matches!(self.input_context, Some(InputContext::NewSessionCwd { .. }));

        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.input_context = None;
                self.path_suggestions.clear();
                self.path_suggestion_cursor = 0;
            }
            KeyCode::Tab if is_cwd && !self.path_suggestions.is_empty() => {
                // Accept highlighted suggestion
                if let Some(suggestion) = self.path_suggestions.get(self.path_suggestion_cursor).cloned() {
                    self.input_buffer = suggestion;
                    if crate::path_complete::is_directory(&self.input_buffer) {
                        if !self.input_buffer.ends_with('/') {
                            self.input_buffer.push('/');
                        }
                    }
                }
                self.refresh_path_suggestions();
            }
            KeyCode::Up | KeyCode::Char('k') if is_cwd && !self.path_suggestions.is_empty() => {
                if self.path_suggestion_cursor == 0 {
                    self.path_suggestion_cursor = self.path_suggestions.len() - 1;
                } else {
                    self.path_suggestion_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') if is_cwd && !self.path_suggestions.is_empty() => {
                self.path_suggestion_cursor =
                    (self.path_suggestion_cursor + 1) % self.path_suggestions.len();
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                if is_cwd {
                    self.refresh_path_suggestions();
                }
            }
            KeyCode::Enter => {
                let mut buffer = self.input_buffer.clone();
                if buffer.trim().is_empty() {
                    return;
                }
                // Expand ~ before passing to process_text_input
                if is_cwd && (buffer == "~" || buffer.starts_with("~/")) {
                    if let Some(home) = dirs::home_dir() {
                        buffer = format!("{}{}", home.display(), &buffer[1..]);
                    }
                }
                self.path_suggestions.clear();
                self.path_suggestion_cursor = 0;
                self.process_text_input(buffer);
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                if is_cwd {
                    self.refresh_path_suggestions();
                }
            }
            _ => {}
        }
    }

    fn refresh_path_suggestions(&mut self) {
        self.path_suggestions = crate::path_complete::complete_path(&self.input_buffer);
        if self.path_suggestion_cursor >= self.path_suggestions.len() {
            self.path_suggestion_cursor = 0;
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
                self.refresh_path_suggestions();
            }
            InputContext::NewSessionCwd { name } => {
                match self.db.get_all_groups() {
                    Ok(groups) if !groups.is_empty() => {
                        // Prepend "Ungrouped" sentinel (id 0)
                        let mut picker = vec![(0i64, "Ungrouped".to_string())];
                        picker.extend(groups);
                        self.picker_cursor = self.hovered_group_picker_index(&picker);
                        self.picker_groups = picker;
                        self.input_mode = InputMode::GroupPicker;
                        self.input_context = Some(InputContext::NewSessionGroup {
                            name,
                            cwd: buffer,
                        });
                    }
                    _ => {
                        // No groups — create ungrouped
                        self.create_session(&name, &buffer, None);
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                    }
                }
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
                let gid = self.picker_groups.get(self.picker_cursor).map(|(g, _)| *g);
                match self.input_context.take() {
                    Some(InputContext::MoveSession { session_id }) => {
                        if let Some(gid) = gid {
                            if let Err(e) = self.db.move_session_to_group(&session_id, gid) {
                                self.status_message =
                                    Some((format!("move failed: {e}"), Instant::now()));
                            }
                        }
                    }
                    Some(InputContext::NewSessionGroup { name, cwd }) => {
                        // gid 0 is the "Ungrouped" sentinel
                        let group = gid.filter(|&id| id != 0);
                        self.create_session(&name, &cwd, group);
                    }
                    other => {
                        self.input_context = other;
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

    /// Return the picker index matching the group the cursor currently sits in
    /// (either a group node itself, or the parent group of a session).  Falls
    /// back to 0 (the "Ungrouped" sentinel when present, or first entry).
    fn hovered_group_picker_index(&mut self, picker: &[(GroupId, String)]) -> usize {
        let target = self.tree_state.selected_target(&self.tree);
        let group_id = match target {
            Some(SelectionTarget::Group(gid)) => Some(gid),
            Some(SelectionTarget::Session(ref sid)) => {
                // Walk the tree to find the parent group of this session
                self.tree.iter().find_map(|node| {
                    if let TreeNode::Group(g) = node {
                        let has_child = g.children.iter().any(|c| {
                            matches!(c, TreeNode::Session(s) if s.session_id == *sid)
                        });
                        if has_child { Some(g.id) } else { None }
                    } else {
                        None
                    }
                })
            }
            None => None,
        };
        match group_id {
            Some(gid) => picker.iter().position(|(id, _)| *id == gid).unwrap_or(0),
            None => 0,
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

    fn create_session(&mut self, name: &str, cwd: &str, group_id: Option<GroupId>) {
        let tmux_name = sanitize_tmux_name(name);
        match self.db.create_nexus_session(name, cwd, &tmux_name) {
            Ok(id) => {
                if let Some(gid) = group_id {
                    if let Err(e) = self.db.assign_session_to_group(&id, gid) {
                        self.status_message =
                            Some((format!("group assign failed: {e}"), Instant::now()));
                    }
                }
                if self.tmux_available {
                    if let Err(e) = self.tmux.launch_claude_session(&tmux_name, cwd) {
                        self.status_message =
                            Some((format!("tmux launch failed: {e}"), Instant::now()));
                        self.refresh_tree();
                        return;
                    }
                    let _ = self.tmux.configure_server();
                }
                // Select the new session in the tree so the interactor picks it up
                self.selection.selected = Some(SelectionTarget::Session(id));
                self.refresh_tree();
                self.sync_interactor_to_selection();
                self.ensure_session_launched();
            }
            Err(e) => {
                self.status_message =
                    Some((format!("create failed: {e}"), Instant::now()));
            }
        }
    }

    /// Launch a detached tmux session in the background for the currently
    /// selected session so the interactor can capture it. Does NOT attach
    /// fullscreen — use `fullscreen_attach` (Alt+F) for that.
    fn ensure_session_launched(&mut self) {
        if !self.tmux_available {
            return;
        }

        let session = match self.cached_selected.as_ref() {
            Some(s) => s.clone(),
            None => return,
        };

        match session.status {
            SessionStatus::Active => {
                // Already running — capture worker handles display
            }
            SessionStatus::Detached => {
                let cwd = match session.cwd.as_ref().map(|p| p.to_string_lossy().to_string()) {
                    Some(c) => c,
                    None => return,
                };
                let tmux_name = match session.tmux_name.as_ref() {
                    Some(n) => n.clone(),
                    None => sanitize_tmux_name(&session.session_id),
                };

                if let Err(e) = self.tmux.launch_claude_session(&tmux_name, &cwd) {
                    self.status_message =
                        Some((format!("tmux launch failed: {e}"), Instant::now()));
                    return;
                }
                let _ = self.tmux.configure_server();
                let _ = self.db.update_session_status(&session.session_id, SessionStatus::Active);
                self.refresh_tree();
                // Re-sync so interactor switches from conversation log to live capture
                self.refresh_cached_selected();
                self.sync_interactor_to_selection();
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
        // Leave ratatui's alternate screen, raw mode, and bracketed paste so tmux can take over
        let _ = crossterm::execute!(std::io::stdout(), DisableBracketedPaste);
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);

        let result = self.tmux.resume_session(tmux_name);

        // Restore ratatui's terminal state
        let _ = crossterm::execute!(std::io::stdout(), EnterAlternateScreen);
        let _ = crossterm::terminal::enable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), EnableBracketedPaste);

        // Force a full redraw — ratatui's internal buffer is stale after tmux
        self.needs_full_redraw = true;

        if let Err(e) = result {
            self.status_message =
                Some((format!("tmux attach failed: {e}"), Instant::now()));
        }

        // Re-sync after fullscreen: the capture worker may have stopped during
        // attachment (error → cleared session), and session status may have
        // changed (e.g., Detached→Active via launch before attach).
        self.refresh_tree();
        self.sync_interactor_to_selection();
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
            // Re-sync interactor — selected session may have changed status
            // (e.g., Active → Detached triggers conversation log transition)
            self.refresh_cached_selected();
            self.sync_interactor_to_selection();
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

/// Compute the exact inner dimensions of the interactor panel.
///
/// Mirrors the layout in `ui.rs`: top_bar (3 rows) + main area, right column
/// is 87% width (tree is 13%), interactor fills full height. Subtract 2 for borders.
fn interactor_inner_size(term_cols: u16, term_rows: u16) -> (u16, u16) {
    let main_height = term_rows.saturating_sub(3); // top_bar = Length(3)
    let right_width = term_cols * 87 / 100; // tree is 13%, right column fills rest
    let inner_cols = right_width.saturating_sub(2);
    let inner_rows = main_height.saturating_sub(2);
    (inner_cols, inner_rows)
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
