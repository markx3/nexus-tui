use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::types::{PanelType, ThemeElement};

/// Render the top status bar.
///
/// Layout: `SYS:ONLINE == SESSIONS:{count} == ACTIVE:{count} == {date}`
pub fn render_top_bar(frame: &mut Frame, area: Rect, session_count: usize, active_count: usize) {
    let date = current_date_string();

    let active_style = if active_count > 0 {
        theme::style_for(ThemeElement::Secondary)
    } else {
        theme::style_for(ThemeElement::Text)
    };

    let status = Line::from(vec![
        Span::styled(" SYS:", theme::style_for(ThemeElement::TopBarLabel)),
        Span::styled("ONLINE", theme::style_for(ThemeElement::Secondary)),
        Span::styled(
            format!(" {} ", theme::SEPARATOR),
            theme::style_for(ThemeElement::TopBarLabel),
        ),
        Span::styled("SESSIONS:", theme::style_for(ThemeElement::TopBarLabel)),
        Span::styled(
            format!("{session_count}"),
            theme::style_for(ThemeElement::TopBarValue),
        ),
        Span::styled(
            format!(" {} ", theme::SEPARATOR),
            theme::style_for(ThemeElement::TopBarLabel),
        ),
        Span::styled("ACTIVE:", theme::style_for(ThemeElement::TopBarLabel)),
        Span::styled(format!("{active_count}"), active_style),
        Span::styled(
            format!(" {} ", theme::SEPARATOR),
            theme::style_for(ThemeElement::TopBarLabel),
        ),
        Span::styled(date, theme::style_for(ThemeElement::TopBarLabel)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme::border_for(PanelType::TopBar))
        .border_style(theme::border_style_for(PanelType::TopBar, true))
        .style(theme::style_for(ThemeElement::Surface));

    let paragraph = Paragraph::new(status).block(block);
    frame.render_widget(paragraph, area);
}

/// Format the current date as `YYYY.MM.DD` using the consolidated time_utils.
fn current_date_string() -> String {
    crate::time_utils::epoch_to_date_display(crate::time_utils::now_epoch())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_top_bar_does_not_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_top_bar(frame, area, 5, 2);
            })
            .unwrap();
    }

    #[test]
    fn render_top_bar_zero_active() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_top_bar(frame, area, 0, 0);
            })
            .unwrap();
    }
}
