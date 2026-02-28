use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::types::PanelType;

/// Render the top status bar.
///
/// Layout: `SYS:ONLINE == SESSIONS:{count} == ACTIVE:{count} == {date}`
pub fn render_top_bar(
    frame: &mut Frame,
    area: Rect,
    session_count: usize,
    active_count: usize,
) {
    let date = current_date_string();

    let active_style = if active_count > 0 {
        theme::style_for(crate::types::ThemeElement::AcidGreen)
    } else {
        theme::style_for(crate::types::ThemeElement::Text)
    };

    let status = Line::from(vec![
        Span::styled(" SYS:", theme::style_for(crate::types::ThemeElement::TopBarLabel)),
        Span::styled("ONLINE", theme::style_for(crate::types::ThemeElement::AcidGreen)),
        Span::styled(
            format!(" {} ", theme::SEPARATOR),
            theme::style_for(crate::types::ThemeElement::TopBarLabel),
        ),
        Span::styled("SESSIONS:", theme::style_for(crate::types::ThemeElement::TopBarLabel)),
        Span::styled(
            format!("{session_count}"),
            theme::style_for(crate::types::ThemeElement::TopBarValue),
        ),
        Span::styled(
            format!(" {} ", theme::SEPARATOR),
            theme::style_for(crate::types::ThemeElement::TopBarLabel),
        ),
        Span::styled("ACTIVE:", theme::style_for(crate::types::ThemeElement::TopBarLabel)),
        Span::styled(format!("{active_count}"), active_style),
        Span::styled(
            format!(" {} ", theme::SEPARATOR),
            theme::style_for(crate::types::ThemeElement::TopBarLabel),
        ),
        Span::styled(date, theme::style_for(crate::types::ThemeElement::TopBarLabel)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(theme::border_for(PanelType::TopBar))
        .border_style(theme::border_style_for(PanelType::TopBar, true))
        .style(theme::style_for(crate::types::ThemeElement::Surface));

    let paragraph = Paragraph::new(status).block(block);
    frame.render_widget(paragraph, area);
}

/// Format the current date as `YYYY.MM.DD` using std SystemTime.
fn current_date_string() -> String {
    // Use UNIX_EPOCH arithmetic to get date without external crate
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Days since epoch
    let days = (secs / 86400) as i64;

    // Civil date from days (algorithm from Howard Hinnant)
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}.{m:02}.{d:02}")
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
