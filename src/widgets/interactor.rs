use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme;
use crate::types::*;

/// Render the session interactor panel.
///
/// Displays either:
/// - Live terminal content (pre-parsed `Text<'static>` from capture worker)
/// - Conversation log (for sessions without a tmux pane)
/// - Empty state ("Select a session" when no session or group selected)
pub fn render_interactor(
    frame: &mut Frame,
    area: Rect,
    content: Option<&SessionContent>,
    session_name: Option<&str>,
    _focused: bool,
    log_scroll_offset: u16,
) {
    // Small-area guard: don't render in tiny areas
    if area.width < 10 || area.height < 3 {
        return;
    }

    let title = match session_name {
        Some(name) => format!(" {} ", name),
        None => " SESSION ".to_string(),
    };

    let block = Block::default()
        .title(Span::styled(
            title,
            theme::style_for(ThemeElement::InteractorTitle),
        ))
        .borders(Borders::ALL)
        .border_set(theme::border_for(PanelType::SessionInteractor))
        .border_style(theme::border_style_for(PanelType::SessionInteractor, false))
        .style(theme::style_for(ThemeElement::Surface));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match content {
        Some(SessionContent::Live(text)) => {
            render_live(frame, inner, text);
        }
        Some(SessionContent::ConversationLog(turns)) => {
            render_conversation_log(frame, inner, turns, log_scroll_offset);
        }
        None => {
            render_empty(frame, inner);
        }
    }
}

/// Render live terminal content from the capture worker.
fn render_live(frame: &mut Frame, area: Rect, text: &Text<'static>) {
    let paragraph = Paragraph::new(text.clone());
    frame.render_widget(paragraph, area);
}

/// Render a conversation log for sessions without a tmux pane.
fn render_conversation_log(
    frame: &mut Frame,
    area: Rect,
    turns: &[ConversationTurn],
    scroll_offset: u16,
) {
    if turns.is_empty() {
        let msg = Paragraph::new("No conversation data available")
            .style(theme::style_for(ThemeElement::Dim))
            .alignment(Alignment::Center);
        frame.render_widget(msg, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    for turn in turns {
        let (role_label, role_style) = match turn.role {
            Role::Human => ("You", theme::style_for(ThemeElement::ConversationHuman)),
            Role::Assistant => ("Claude", theme::style_for(ThemeElement::ConversationAssistant)),
        };

        // Role header
        lines.push(Line::from(Span::styled(
            format!("--- {role_label} ---"),
            role_style,
        )));

        // Content lines
        for content_line in turn.content.lines() {
            lines.push(Line::from(Span::styled(
                content_line.to_string(),
                theme::style_for(ThemeElement::Text),
            )));
        }

        // Blank separator
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));
    frame.render_widget(paragraph, area);
}

/// Render the empty state when no session is selected or a group node is selected.
fn render_empty(frame: &mut Frame, area: Rect) {
    let msg = Paragraph::new("Select a session")
        .style(theme::style_for(ThemeElement::Dim))
        .alignment(Alignment::Center);
    frame.render_widget(msg, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_interactor_empty_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_interactor(frame, area, None, None, false, 0);
            })
            .unwrap();
    }

    #[test]
    fn render_interactor_live_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let text = Text::raw("Hello from terminal");
        let content = SessionContent::Live(text);
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_interactor(frame, area, Some(&content), Some("test-session"), false, 0);
            })
            .unwrap();
    }

    #[test]
    fn render_interactor_conversation_log_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let turns = vec![
            ConversationTurn {
                role: Role::Human,
                content: "Hello".to_string(),
            },
            ConversationTurn {
                role: Role::Assistant,
                content: "Hi there!".to_string(),
            },
        ];
        let content = SessionContent::ConversationLog(turns);
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_interactor(frame, area, Some(&content), Some("old-session"), false, 0);
            })
            .unwrap();
    }

    #[test]
    fn render_interactor_empty_conversation_log() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let content = SessionContent::ConversationLog(vec![]);
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_interactor(frame, area, Some(&content), None, false, 0);
            })
            .unwrap();
    }

    #[test]
    fn render_interactor_small_area_no_panic() {
        let backend = TestBackend::new(5, 2);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_interactor(frame, area, None, None, false, 0);
            })
            .unwrap();
    }

    #[test]
    fn render_interactor_zero_area_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 0, 0);
                render_interactor(frame, area, None, None, false, 0);
            })
            .unwrap();
    }

    #[test]
    fn render_interactor_with_scroll_offset() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let turns: Vec<ConversationTurn> = (0..50)
            .map(|i| ConversationTurn {
                role: if i % 2 == 0 { Role::Human } else { Role::Assistant },
                content: format!("Turn {i}"),
            })
            .collect();
        let content = SessionContent::ConversationLog(turns);
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_interactor(frame, area, Some(&content), None, false, 10);
            })
            .unwrap();
    }
}
