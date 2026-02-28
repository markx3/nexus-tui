use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::types::{PanelType, ThemeElement, TmuxSessionStatus, TmuxWindowInfo};

/// Block characters used for the activity gauge, ordered by fill level.
const GAUGE_CHARS: [char; 8] = ['\u{258f}', '\u{258e}', '\u{258d}', '\u{258c}', '\u{258b}', '\u{258a}', '\u{2589}', '\u{2588}'];
// ▏ ▎ ▍ ▌ ▋ ▊ ▉ █

/// Width of the gauge bar in characters.
const GAUGE_WIDTH: usize = 8;

/// Render the activity strip showing tmux session gauges.
///
/// For each `TmuxWindowInfo`:
/// - Running sessions get a full gauge in ACID_GREEN
/// - Idle sessions get a partial gauge in DIM
/// - If no windows, show a placeholder message
pub fn render_activity_strip(
    frame: &mut Frame,
    area: Rect,
    windows: &[TmuxWindowInfo],
) {
    let block = Block::default()
        .title(Span::styled(
            " ACTIVITY ",
            theme::style_for(ThemeElement::NeonCyan),
        ))
        .borders(Borders::ALL)
        .border_set(theme::border_for(PanelType::ActivityStrip))
        .border_style(theme::border_style_for(PanelType::ActivityStrip, false))
        .style(theme::style_for(ThemeElement::Surface));

    if windows.is_empty() {
        let content = Paragraph::new("No active sessions")
            .style(theme::style_for(ThemeElement::Dim))
            .block(block);
        frame.render_widget(content, area);
        return;
    }

    // Build a single-line representation of all windows (fits in the 1-line inner area)
    let mut spans: Vec<Span> = Vec::new();

    for (i, window) in windows.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", theme::style_for(ThemeElement::Dim)));
        }

        let (gauge, style) = match window.status {
            TmuxSessionStatus::Running => {
                // Full gauge
                let bar: String = std::iter::repeat_n(GAUGE_CHARS[7], GAUGE_WIDTH).collect();
                (bar, theme::style_for(ThemeElement::ActivityGauge))
            }
            TmuxSessionStatus::Idle => {
                // Partial gauge (3/8 fill)
                let filled = GAUGE_WIDTH / 3;
                let partial_idx = 3; // ▌
                let mut bar: String = std::iter::repeat_n(GAUGE_CHARS[7], filled).collect();
                bar.push(GAUGE_CHARS[partial_idx]);
                let remaining = GAUGE_WIDTH.saturating_sub(filled + 1);
                for _ in 0..remaining {
                    bar.push(' ');
                }
                (bar, theme::style_for(ThemeElement::IdleSession))
            }
        };

        spans.push(Span::styled(
            format!("{} ", window.window_name),
            theme::style_for(ThemeElement::Dim),
        ));
        spans.push(Span::styled(gauge, style));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).block(block);
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_activity_strip_empty() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_activity_strip(frame, area, &[]);
            })
            .unwrap();
    }

    #[test]
    fn render_activity_strip_with_windows() {
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let windows = mock::mock_tmux_windows();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_activity_strip(frame, area, &windows);
            })
            .unwrap();
    }

    #[test]
    fn gauge_chars_count() {
        // Verify we have exactly 8 block fill levels
        assert_eq!(GAUGE_CHARS.len(), 8);
    }
}
