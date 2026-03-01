use std::sync::mpsc;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::text::Text;

use crate::conversation;
use crate::tmux::{key_event_to_send_args, TmuxManager};
use crate::types::*;

/// State for the session interactor panel.
///
/// Owns the capture worker communication channels, current display content,
/// and a cloned TmuxManager for send_keys/resize/paste operations.
pub struct InteractorState {
    tmux: TmuxManager,
    content_rx: mpsc::Receiver<Option<Text<'static>>>,
    session_tx: mpsc::Sender<String>,
    pub current_content: Option<SessionContent>,
    last_resize: (u16, u16),
    pub current_session_name: Option<String>,
    pub log_scroll_offset: u16,
}

impl InteractorState {
    pub fn new(
        tmux: TmuxManager,
        content_rx: mpsc::Receiver<Option<Text<'static>>>,
        session_tx: mpsc::Sender<String>,
    ) -> Self {
        Self {
            tmux,
            content_rx,
            session_tx,
            current_content: None,
            last_resize: (0, 0),
            current_session_name: None,
            log_scroll_offset: 0,
        }
    }

    /// Poll for new content from the capture worker (non-blocking).
    ///
    /// Returns `true` if content was updated (triggers a redraw).
    pub fn poll_content(&mut self) -> bool {
        let mut updated = false;
        // Drain all pending messages, keep the latest
        while let Ok(msg) = self.content_rx.try_recv() {
            match msg {
                Some(text) => {
                    self.current_content = Some(SessionContent::Live(text));
                    updated = true;
                }
                None => {
                    // Session gone — will transition to conversation log
                    // on next reconciliation cycle
                    self.current_content = None;
                    updated = true;
                }
            }
        }
        updated
    }

    /// Switch to a different session.
    ///
    /// Tells the capture worker which session to capture. If the session has no
    /// tmux pane, loads the conversation log instead.
    pub fn switch_session(
        &mut self,
        session: &SessionSummary,
    ) {
        self.log_scroll_offset = 0;
        self.current_session_name = Some(session.display_name.clone());

        if session.status == SessionStatus::Active {
            // Active session — tell capture worker to start capturing
            if let Some(ref tmux_name) = session.tmux_name {
                let _ = self.session_tx.send(tmux_name.clone());
            }
        } else {
            // Dead/detached — show conversation log
            self.load_conversation_log(session);
            // Tell capture worker to stop capturing (empty name)
            let _ = self.session_tx.send(String::new());
        }
    }

    /// Clear the interactor (e.g., when a group node is selected).
    pub fn clear(&mut self) {
        self.current_content = None;
        self.current_session_name = None;
        self.log_scroll_offset = 0;
        let _ = self.session_tx.send(String::new());
    }

    /// Resize the tmux pane to match the interactor panel dimensions.
    ///
    /// Only sends the resize command if dimensions actually changed.
    pub fn resize_if_needed(&mut self, cols: u16, rows: u16) {
        if (cols, rows) != self.last_resize && cols > 0 && rows > 0 {
            if let Some(ref name) = self.current_session_name {
                // Find the tmux name — for now, use session_tx's last value
                // The actual tmux name is sent via the capture worker channel
                let _ = self.tmux.resize_pane(name, cols, rows);
            }
            self.last_resize = (cols, rows);
        }
    }

    /// Load conversation log for a dead/detached session.
    fn load_conversation_log(&mut self, session: &SessionSummary) {
        if let Some(ref path) = session.jsonl_path {
            let turns = conversation::parse_conversation(path, 100);
            if turns.is_empty() {
                self.current_content = Some(SessionContent::ConversationLog(Vec::new()));
            } else {
                self.current_content = Some(SessionContent::ConversationLog(turns));
            }
        } else {
            self.current_content = Some(SessionContent::ConversationLog(Vec::new()));
        }
    }

    /// Scroll conversation log up.
    pub fn scroll_up(&mut self, amount: u16) {
        self.log_scroll_offset = self.log_scroll_offset.saturating_sub(amount);
    }

    /// Scroll conversation log down.
    pub fn scroll_down(&mut self, amount: u16) {
        self.log_scroll_offset = self.log_scroll_offset.saturating_add(amount);
    }

    /// Route an input event: Alt→NexusCommand, forward→tmux, or ignored.
    ///
    /// `current_tmux_name` is the tmux session name to forward keys to.
    /// When `None` (no active tmux pane), non-Alt keys are ignored.
    pub fn route_event(
        &mut self,
        event: &Event,
        current_tmux_name: Option<&str>,
    ) -> RouteResult {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                // Alt+key → NexusCommand
                if key.modifiers.contains(KeyModifiers::ALT) {
                    return match key.code {
                        KeyCode::Char('j') => RouteResult::NexusCommand(NexusCommand::CursorDown),
                        KeyCode::Char('k') => RouteResult::NexusCommand(NexusCommand::CursorUp),
                        KeyCode::Enter => RouteResult::NexusCommand(NexusCommand::ToggleExpand),
                        KeyCode::Char('n') => RouteResult::NexusCommand(NexusCommand::NewSession),
                        KeyCode::Char('d') => RouteResult::NexusCommand(NexusCommand::DeleteSelected),
                        KeyCode::Char('r') => RouteResult::NexusCommand(NexusCommand::RenameSelected),
                        KeyCode::Char('m') => RouteResult::NexusCommand(NexusCommand::MoveSession),
                        KeyCode::Char('g') => RouteResult::NexusCommand(NexusCommand::NewGroup),
                        KeyCode::Char('x') => RouteResult::NexusCommand(NexusCommand::KillTmux),
                        KeyCode::Char('f') => RouteResult::NexusCommand(NexusCommand::FullscreenAttach),
                        KeyCode::Char('h') | KeyCode::Char('?') => {
                            RouteResult::NexusCommand(NexusCommand::ToggleHelp)
                        }
                        KeyCode::Char('q') => RouteResult::NexusCommand(NexusCommand::Quit),
                        KeyCode::Char('H') => {
                            RouteResult::NexusCommand(NexusCommand::ToggleDeadSessions)
                        }
                        _ => RouteResult::Ignored,
                    };
                }

                // Non-Alt key → forward to tmux if we have an active pane
                if let Some(tmux_name) = current_tmux_name {
                    if let Some(args) = key_event_to_send_args(key) {
                        let _ = self.tmux.send_keys(tmux_name, &args);
                        return RouteResult::Forwarded;
                    }
                }

                // No active tmux pane — handle scroll for conversation log
                if matches!(self.current_content, Some(SessionContent::ConversationLog(_))) {
                    match key.code {
                        KeyCode::Up => { self.scroll_up(1); return RouteResult::Forwarded; }
                        KeyCode::Down => { self.scroll_down(1); return RouteResult::Forwarded; }
                        KeyCode::PageUp => { self.scroll_up(10); return RouteResult::Forwarded; }
                        KeyCode::PageDown => { self.scroll_down(10); return RouteResult::Forwarded; }
                        _ => {}
                    }
                }

                RouteResult::Ignored
            }
            Event::Paste(text) => {
                // Paste → load-buffer + paste-buffer (with 1MB limit)
                if let Some(tmux_name) = current_tmux_name {
                    if text.len() <= 1_048_576 {
                        let _ = self.tmux.load_buffer_and_paste(tmux_name, text);
                        return RouteResult::Forwarded;
                    }
                }
                RouteResult::Ignored
            }
            _ => RouteResult::Ignored,
        }
    }
}
