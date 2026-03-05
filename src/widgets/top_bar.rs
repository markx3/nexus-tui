use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::types::{PanelType, ThemeElement};

/// Render the top status bar and return the screen rect of the theme label (for click hit-testing).
///
/// Layout: `SYS:ONLINE == SESSIONS:{count} == ACTIVE:{count} == {date} == THEME:{name}`
pub fn render_top_bar(
    frame: &mut Frame,
    area: Rect,
    session_count: usize,
    active_count: usize,
    update_available: bool,
) -> Rect {
    let date = current_date_string();

    let active_style = if active_count > 0 {
        theme::style_for(ThemeElement::Secondary)
    } else {
        theme::style_for(ThemeElement::Text)
    };

    let theme_text = format!("THEME:{}", theme::current_name());

    let mut spans = vec![
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
        Span::styled(&date, theme::style_for(ThemeElement::TopBarLabel)),
        Span::styled(
            format!(" {} ", theme::SEPARATOR),
            theme::style_for(ThemeElement::TopBarLabel),
        ),
        Span::styled(&theme_text, theme::style_for(ThemeElement::Accent)),
    ];
    if update_available {
        spans.push(Span::styled(
            format!(" {} ", theme::SEPARATOR),
            theme::style_for(ThemeElement::TopBarLabel),
        ));
        spans.push(Span::styled(
            "UPDATE",
            theme::style_for(ThemeElement::Hazard),
        ));
    }
    let status = Line::from(spans);

    // Compute the screen rect of the theme label for mouse hit-testing
    let inner = Block::default().borders(Borders::ALL).inner(area);
    let preceding_width: usize = status.spans[..status.spans.len() - 1]
        .iter()
        .map(|s| s.width())
        .sum();
    let theme_rect = Rect {
        x: inner.x + preceding_width as u16,
        y: inner.y,
        width: theme_text.len() as u16,
        height: 1,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme::border_for(PanelType::TopBar))
        .border_style(theme::border_style_for(PanelType::TopBar, true))
        .style(theme::style_for(ThemeElement::Surface));

    let paragraph = Paragraph::new(status).block(block);
    frame.render_widget(paragraph, area);

    theme_rect
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
                let _ = render_top_bar(frame, area, 5, 2, false);
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
                let _ = render_top_bar(frame, area, 0, 0, false);
            })
            .unwrap();
    }
}
