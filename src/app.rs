use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::DefaultTerminal;
use tachyonfx::Effect;

use crate::mock;
use crate::theme;
use crate::types::*;
use crate::ui;
use crate::widgets::radar_state::RadarState;
use crate::widgets::tree_state::{TreeAction, TreeState};

const TICK_RATE: Duration = Duration::from_millis(16);

pub struct App {
    pub should_quit: bool,
    pub(crate) dirty: bool,
    pub(crate) boot_done: bool,
    last_tick: Instant,
    pub(crate) boot_effects: Vec<Effect>,
    pub(crate) tree_state: TreeState,
    pub(crate) radar_state: RadarState,
    pub(crate) selection: SelectionState,
    pub(crate) mock_tree: Vec<TreeNode>,
}

impl App {
    pub fn new() -> Self {
        let mock_tree = mock::mock_tree();
        let tree_state = TreeState::new(&mock_tree);
        let mut radar_state = RadarState::new();
        radar_state.compute_blips(&mock_tree);
        let selection = SelectionState::default();

        Self {
            should_quit: false,
            dirty: true,
            boot_done: false,
            last_tick: Instant::now(),
            boot_effects: theme::create_boot_effects(),
            tree_state,
            radar_state,
            selection,
            mock_tree,
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_tick);
            self.last_tick = now;

            // Advance radar sweep
            self.radar_state.advance_sweep(elapsed.as_secs_f64());

            // Always redraw: radar sweep animates continuously
            {
                terminal.draw(|frame| ui::draw(frame, &mut self, elapsed))?;
                self.dirty = false;
            }

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
                // Switch focus between tree and radar
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
                if let Some(action) = self.tree_state.handle_key(key, &self.mock_tree) {
                    self.handle_tree_action(action);
                }
            }
            FocusPanel::Radar => {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.radar_state.move_cursor_next();
                        // Sync selection
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
                        if let Some(id) = self.radar_state.selected_session() {
                            self.selection.selected =
                                Some(SelectionTarget::Session(id.to_string()));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_tree_action(&mut self, action: TreeAction) {
        match action {
            TreeAction::Select(target) => {
                // Sync radar cursor if selecting a session
                if let SelectionTarget::Session(ref id) = target {
                    self.radar_state.select_by_session_id(id);
                }
                self.selection.selected = Some(target);
            }
            TreeAction::ToggleExpand(_) => {
                // Recompute radar blips after expand/collapse
                self.radar_state.compute_blips(&self.mock_tree);
            }
            TreeAction::ScrollDown | TreeAction::ScrollUp => {
                // Update selection to track cursor
                if let Some(target) = self.tree_state.selected_target(&self.mock_tree) {
                    if let SelectionTarget::Session(ref id) = target {
                        self.radar_state.select_by_session_id(id);
                    }
                    self.selection.selected = Some(target);
                }
            }
            _ => {}
        }
    }
}
