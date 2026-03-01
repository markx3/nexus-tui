use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::types::{PanelType, SessionSummary, ThemeElement};

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

    let id_short = if session.session_id.len() > 8 {
        &session.session_id[session.session_id.len() - 8..]
    } else {
        &session.session_id
    };

    let cwd_display = session
        .cwd
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "-".to_string());

    let status_display = session.status.as_str();
    let origin_display = session.created_by.as_str();
    let tmux_display = session.tmux_name.as_deref().unwrap_or("-");

    let mut lines: Vec<Line> = vec![
        detail_line("Session ID", id_short, label_style, value_style),
        detail_line("Name", &session.display_name, label_style, value_style),
        detail_line("CWD", &cwd_display, label_style, value_style),
        detail_line("Status", status_display, label_style, value_style),
        detail_line("Origin", origin_display, label_style, value_style),
        detail_line("Tmux", tmux_display, label_style, value_style),
        detail_line("Last active", &session.last_active, label_style, value_style),
        detail_line("Created", &session.created_at, label_style, value_style),
    ];

    // Blank line before action bar
    lines.push(Line::from(""));

    // Action bar
    let action_key_style = theme::style_for(ThemeElement::NeonCyan);
    let action_label_style = theme::style_for(ThemeElement::Dim);

    lines.push(Line::from(vec![
        Span::styled("[Enter]", action_key_style),
        Span::styled(" Resume  ", action_label_style),
        Span::styled("[d]", action_key_style),
        Span::styled(" Delete  ", action_label_style),
        Span::styled("[m]", action_key_style),
        Span::styled(" Move  ", action_label_style),
        Span::styled("[r]", action_key_style),
        Span::styled(" Rename", action_label_style),
    ]));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn detail_line(
    label: &str,
    value: &str,
    label_style: ratatui::style::Style,
    value_style: ratatui::style::Style,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), label_style),
        Span::styled(value.to_owned(), value_style),
    ])
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
