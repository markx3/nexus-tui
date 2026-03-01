use std::sync::mpsc;

use ratatui::text::Text;

use crate::conversation;
use crate::tmux::TmuxManager;
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

    /// Get a reference to the TmuxManager for send_keys and paste operations.
    pub fn tmux(&self) -> &TmuxManager {
        &self.tmux
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
}
