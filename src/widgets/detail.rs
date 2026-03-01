use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::types::{PanelType, SessionSummary, ThemeElement};

/// Render the compact detail panel (~1/6 height of right column).
///
/// Shows: name, cwd, status, tmux name — one or two lines.
pub fn render_detail(
    frame: &mut Frame,
    area: Rect,
    session: Option<&SessionSummary>,
    focused: bool,
) {
    let block = Block::default()
        .title(Span::styled(
            " DETAIL ",
            theme::style_for(ThemeElement::NeonCyan),
        ))
        .borders(Borders::ALL)
        .border_set(theme::border_for(PanelType::Detail))
        .border_style(theme::border_style_for(PanelType::Detail, focused))
        .style(theme::style_for(ThemeElement::Surface));

    let Some(session) = session else {
        let content = Paragraph::new("Select a session to view details")
            .style(theme::style_for(ThemeElement::Dim))
            .block(block);
        frame.render_widget(content, area);
        return;
    };

    let label_style = theme::style_for(ThemeElement::DetailLabel);
    let value_style = theme::style_for(ThemeElement::DetailValue);
    let status_style = match session.status {
        crate::types::SessionStatus::Active => theme::style_for(ThemeElement::AcidGreen),
        crate::types::SessionStatus::Detached => theme::style_for(ThemeElement::Hazard),
        crate::types::SessionStatus::Dead => theme::style_for(ThemeElement::Dim),
    };

    let cwd_display = session
        .cwd
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "-".to_string());
    let tmux_display = session.tmux_name.as_deref().unwrap_or("-");

    let lines: Vec<Line> = vec![
        // Line 1: Name + Status
        Line::from(vec![
            Span::styled(&session.display_name, value_style),
            Span::styled("  ", label_style),
            Span::styled(session.status.as_str(), status_style),
        ]),
        // Line 2: CWD + Tmux name
        Line::from(vec![
            Span::styled("cwd: ", label_style),
            Span::styled(cwd_display, value_style),
            Span::styled("  tmux: ", label_style),
            Span::styled(tmux_display.to_string(), value_style),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_detail_none_does_not_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_detail(frame, area, None, false);
            })
            .unwrap();
    }

    #[test]
    fn render_detail_with_session_does_not_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = mock::mock_tree();
        let session = find_first_session(&tree);
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_detail(frame, area, session.as_ref(), true);
            })
            .unwrap();
    }

    #[test]
    fn render_detail_compact_in_small_area() {
        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = mock::mock_tree();
        let session = find_first_session(&tree);
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_detail(frame, area, session.as_ref(), false);
            })
            .unwrap();
    }

    fn find_first_session(tree: &[crate::types::TreeNode]) -> Option<SessionSummary> {
        for node in tree {
            match node {
                crate::types::TreeNode::Session(s) => return Some(s.clone()),
                crate::types::TreeNode::Group(g) => {
                    if let Some(s) = find_first_session(&g.children) {
                        return Some(s);
                    }
                }
            }
        }
        None
    }
}
