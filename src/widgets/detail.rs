use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::types::{PanelType, SessionSummary, ThemeElement};

/// Render session metadata in the detail panel.
///
/// When `session` is `None`, shows a placeholder message.
/// When present, renders label-value pairs and an action bar.
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

    // Inner width for content (minus block borders)
    let inner_width = area.width.saturating_sub(2) as usize;

    // Truncate session ID to last 8 chars
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

    let branch_display = session
        .git_branch
        .as_deref()
        .unwrap_or("-");

    let model_display = session
        .model
        .as_deref()
        .unwrap_or("-");

    let tokens_display = format!(
        "{} / {}",
        format_number(session.input_tokens),
        format_number(session.output_tokens),
    );

    let message_count_str = session.message_count.to_string();
    let subagent_count_str = session.subagent_count.to_string();

    let mut lines: Vec<Line> = vec![
        detail_line("Session ID", id_short, label_style, value_style),
        detail_line("Project", &session.project_dir, label_style, value_style),
        detail_line("CWD", &cwd_display, label_style, value_style),
        detail_line("Branch", branch_display, label_style, value_style),
        detail_line("Model", model_display, label_style, value_style),
        detail_line("Messages", &message_count_str, label_style, value_style),
        detail_line("Tokens", &tokens_display, label_style, value_style),
        detail_line("Subagents", &subagent_count_str, label_style, value_style),
        detail_line("Last active", &session.last_active, label_style, value_style),
    ];

    // First message (truncated, dim italic)
    if let Some(ref msg) = session.first_message {
        let truncated = if msg.len() > inner_width.saturating_sub(2) {
            format!("{}...", &msg[..inner_width.saturating_sub(5)])
        } else {
            msg.clone()
        };
        lines.push(Line::from(Span::styled(
            truncated,
            theme::style_for(ThemeElement::Dim).add_modifier(Modifier::ITALIC),
        )));
    }

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

/// Build a single label: value line (owned).
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

/// Format a u64 with comma-separated thousands (e.g. 125000 -> "125,000").
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
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
    fn format_number_thousands() {
        assert_eq!(format_number(125_000), "125,000");
        assert_eq!(format_number(89_000), "89,000");
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1_000_000), "1,000,000");
        assert_eq!(format_number(42), "42");
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
