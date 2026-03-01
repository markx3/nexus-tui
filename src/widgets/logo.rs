use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::types::*;

// ---------------------------------------------------------------------------
// Animation frames — 12 frames, each 7 lines x 16 chars
// Agent particles (∙ ◆ ·) orbit around central nexus (◉)
// ---------------------------------------------------------------------------

const FRAME_0: &[&str] = &[
    "                ",
    "  ·         ◆   ",
    "     ∙  ─┼─    ",
    "  ◆  ── ◉ ── · ",
    "     ◆  ─┼─    ",
    "  ∙         ·   ",
    "                ",
];

const FRAME_1: &[&str] = &[
    "        ·       ",
    "  ◆        ∙   ",
    "      · ─┼─    ",
    "  ·  ── ◉ ── ◆ ",
    "     ∙  ─┼─    ",
    "  ·        ◆   ",
    "                ",
];

const FRAME_2: &[&str] = &[
    "  ·             ",
    "  ∙    ◆   ·   ",
    "     ◆  ─┼─    ",
    "  ·  ── ◉ ── ∙ ",
    "      · ─┼─    ",
    "           ◆   ",
    "        ·       ",
];

const FRAME_3: &[&str] = &[
    "     ·          ",
    "        ∙  ◆   ",
    "  ◆  ·  ─┼─    ",
    "  ∙  ── ◉ ── · ",
    "        ─┼─ ·  ",
    "  ◆            ",
    "           ·    ",
];

const FRAME_4: &[&str] = &[
    "           ·    ",
    "  ·   ◆        ",
    "  ∙     ─┼─ ·  ",
    "  ◆  ── ◉ ── ∙ ",
    "  ·     ─┼─    ",
    "     ◆     ·   ",
    "                ",
];

const FRAME_5: &[&str] = &[
    "  ◆             ",
    "     ·     ∙   ",
    "  ·  ◆  ─┼─    ",
    "     ── ◉ ── ◆ ",
    "  ∙  ·  ─┼─    ",
    "        ·      ",
    "           ◆    ",
];

const FRAME_6: &[&str] = &[
    "        ◆       ",
    "  ∙        ·   ",
    "  ·  ∙  ─┼─    ",
    "  ◆  ── ◉ ── · ",
    "     ◆  ─┼─    ",
    "  ·        ∙   ",
    "                ",
];

const FRAME_7: &[&str] = &[
    "  ·        ·    ",
    "     ◆         ",
    "  ◆  ·  ─┼─ ∙ ",
    "     ── ◉ ── ◆ ",
    "  ·     ─┼─    ",
    "  ∙        ◆   ",
    "                ",
];

const FRAME_8: &[&str] = &[
    "           ◆    ",
    "  ◆   ·        ",
    "     ∙  ─┼─ ·  ",
    "  ·  ── ◉ ── ∙ ",
    "  ◆  ·  ─┼─    ",
    "           ·   ",
    "  ·             ",
];

const FRAME_9: &[&str] = &[
    "                ",
    "  ·   ◆    ·   ",
    "  ◆  ∙  ─┼─    ",
    "     ── ◉ ── · ",
    "  ·  ◆  ─┼─    ",
    "  ∙        ◆   ",
    "        ·       ",
];

const FRAME_10: &[&str] = &[
    "     ◆          ",
    "  ·        ◆   ",
    "  ∙  ·  ─┼─    ",
    "  ◆  ── ◉ ── ∙ ",
    "     ·  ─┼─ ◆  ",
    "  ·            ",
    "        ·       ",
];

const FRAME_11: &[&str] = &[
    "        ·       ",
    "  ◆   ∙        ",
    "  ·  ◆  ─┼─ ·  ",
    "     ── ◉ ── ◆ ",
    "  ∙     ─┼─    ",
    "     ·     ◆   ",
    "  ·             ",
];

const LOGO_FRAMES: &[&[&str]] = &[
    FRAME_0, FRAME_1, FRAME_2, FRAME_3, FRAME_4, FRAME_5,
    FRAME_6, FRAME_7, FRAME_8, FRAME_9, FRAME_10, FRAME_11,
];

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

/// Render the animated logo panel.
pub fn render_logo(frame: &mut Frame, area: Rect, frame_index: usize) {
    let block = Block::default()
        .title(Span::styled(
            " ◉ NEXUS ",
            theme::style_for(ThemeElement::LogoNexus),
        ))
        .borders(Borders::ALL)
        .border_set(theme::border_for(PanelType::Logo))
        .border_style(theme::border_style_for(PanelType::Logo, false))
        .style(theme::style_for(ThemeElement::Surface));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let art = LOGO_FRAMES[frame_index % LOGO_FRAMES.len()];
    let agent_style = theme::style_for(ThemeElement::LogoAgent);
    let nexus_style = theme::style_for(ThemeElement::LogoNexus);

    let lines: Vec<Line> = art
        .iter()
        .take(inner.height as usize)
        .map(|line| {
            let spans: Vec<Span> = line
                .chars()
                .map(|ch| match ch {
                    '◉' => Span::styled(ch.to_string(), nexus_style),
                    '∙' | '◆' | '·' | '─' | '│' | '┼' => {
                        Span::styled(ch.to_string(), agent_style)
                    }
                    _ => Span::styled(ch.to_string(), Style::default()),
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_logo_no_panic() {
        let backend = TestBackend::new(20, 9);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_logo(frame, area, 0);
            })
            .unwrap();
    }

    #[test]
    fn render_logo_zero_area() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 0, 0);
                render_logo(frame, area, 0);
            })
            .unwrap();
    }

    #[test]
    fn render_logo_all_frames() {
        let backend = TestBackend::new(20, 9);
        let mut terminal = Terminal::new(backend).unwrap();
        for i in 0..LOGO_FRAMES.len() {
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    render_logo(frame, area, i);
                })
                .unwrap();
        }
    }

    #[test]
    fn render_logo_wraps_frame_index() {
        let backend = TestBackend::new(20, 9);
        let mut terminal = Terminal::new(backend).unwrap();
        // frame_index far beyond frame count should not panic
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_logo(frame, area, 999_999);
            })
            .unwrap();
    }

    #[test]
    fn render_logo_small_area() {
        let backend = TestBackend::new(10, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_logo(frame, area, 5);
            })
            .unwrap();
    }

    #[test]
    fn logo_frames_all_have_seven_lines() {
        for (i, art) in LOGO_FRAMES.iter().enumerate() {
            assert_eq!(art.len(), 7, "Frame {i} has {} lines, expected 7", art.len());
        }
    }
}
