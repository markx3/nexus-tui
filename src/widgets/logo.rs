use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::types::*;

// ---------------------------------------------------------------------------
// Procedural agent definitions
// ---------------------------------------------------------------------------

struct Agent {
    base_angle: f64,
    radius: f64,
    speed: f64,
    symbol: char,
}

const AGENTS: &[Agent] = &[
    // Outer ring — slow, wide orbit
    Agent { base_angle: 0.0,   radius: 0.95, speed: 0.20, symbol: '◆' },
    Agent { base_angle: 1.26,  radius: 0.90, speed: 0.22, symbol: '·' },
    Agent { base_angle: 2.51,  radius: 0.92, speed: 0.18, symbol: '·' },
    Agent { base_angle: 3.77,  radius: 0.88, speed: 0.24, symbol: '◆' },
    Agent { base_angle: 5.03,  radius: 0.93, speed: 0.21, symbol: '·' },
    // Mid ring — moderate speed
    Agent { base_angle: 0.52,  radius: 0.65, speed: 0.35, symbol: '∙' },
    Agent { base_angle: 1.57,  radius: 0.70, speed: 0.38, symbol: '◆' },
    Agent { base_angle: 2.62,  radius: 0.60, speed: 0.42, symbol: '∙' },
    Agent { base_angle: 3.67,  radius: 0.68, speed: 0.36, symbol: '·' },
    Agent { base_angle: 4.71,  radius: 0.72, speed: 0.40, symbol: '∙' },
    // Inner ring — fast, tight orbit
    Agent { base_angle: 0.79,  radius: 0.35, speed: 0.55, symbol: '∙' },
    Agent { base_angle: 2.36,  radius: 0.30, speed: 0.60, symbol: '·' },
    Agent { base_angle: 3.93,  radius: 0.38, speed: 0.52, symbol: '∙' },
    Agent { base_angle: 5.50,  radius: 0.33, speed: 0.58, symbol: '·' },
];

// ---------------------------------------------------------------------------
// Procedural frame generation
// ---------------------------------------------------------------------------

fn generate_frame(width: usize, height: usize, frame_index: usize) -> Vec<Vec<char>> {
    let mut grid = vec![vec![' '; width]; height];

    if width < 5 || height < 3 {
        return grid;
    }

    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;

    // Place crosshair at center
    let cxi = cx as usize;
    let cyi = cy as usize;

    if cyi < height && cxi < width {
        grid[cyi][cxi] = '◉';
    }
    // Horizontal arms
    if cyi < height {
        if cxi >= 2 && cxi + 2 < width {
            grid[cyi][cxi - 2] = '─';
            grid[cyi][cxi - 1] = '─';
            grid[cyi][cxi + 1] = '─';
            grid[cyi][cxi + 2] = '─';
        }
    }
    // Vertical arms + crosshair junction
    if cxi < width {
        if cyi >= 1 && cyi + 1 < height {
            grid[cyi - 1][cxi] = '┼';
            grid[cyi + 1][cxi] = '┼';
        }
        // Extended vertical
        if cxi >= 1 && cxi + 1 < width && cyi >= 1 && cyi + 1 < height {
            grid[cyi - 1][cxi - 1] = '─';
            grid[cyi - 1][cxi + 1] = '─';
            grid[cyi + 1][cxi - 1] = '─';
            grid[cyi + 1][cxi + 1] = '─';
        }
    }

    // Place agents
    let scale_x = (width as f64 / 2.0) - 1.0;
    let scale_y = (height as f64 / 2.0) - 0.5;
    let t = frame_index as f64;

    for agent in AGENTS {
        let angle = agent.base_angle + t * agent.speed;
        let x = cx + agent.radius * scale_x * angle.cos();
        let y = cy + agent.radius * scale_y * angle.sin();

        let xi = x.round() as isize;
        let yi = y.round() as isize;

        if xi >= 0 && yi >= 0 {
            let xi = xi as usize;
            let yi = yi as usize;
            if xi < width && yi < height && grid[yi][xi] == ' ' {
                grid[yi][xi] = agent.symbol;
            }
        }
    }

    grid
}

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

    let w = inner.width as usize;
    let h = inner.height as usize;
    let grid = generate_frame(w, h, frame_index);

    let agent_style = theme::style_for(ThemeElement::LogoAgent);
    let nexus_style = theme::style_for(ThemeElement::LogoNexus);

    let lines: Vec<Line> = grid
        .iter()
        .map(|row| {
            let spans: Vec<Span> = row
                .iter()
                .map(|&ch| match ch {
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
        for i in 0..24 {
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
    fn generate_frame_places_nexus_at_center() {
        let grid = generate_frame(16, 7, 0);
        // Center should have the nexus symbol
        let cx = 8;
        let cy = 3;
        assert_eq!(grid[cy][cx], '◉');
    }

    #[test]
    fn generate_frame_has_crosshair() {
        let grid = generate_frame(16, 7, 0);
        let cx = 8;
        let cy = 3;
        // Horizontal arms
        assert_eq!(grid[cy][cx - 1], '─');
        assert_eq!(grid[cy][cx + 1], '─');
        // Vertical arms
        assert_eq!(grid[cy - 1][cx], '┼');
        assert_eq!(grid[cy + 1][cx], '┼');
    }

    #[test]
    fn generate_frame_tiny_grid() {
        // Should not panic on very small grids
        let grid = generate_frame(3, 2, 0);
        assert_eq!(grid.len(), 2);
        assert_eq!(grid[0].len(), 3);
    }

    #[test]
    fn generate_frame_agents_move() {
        let grid0 = generate_frame(20, 9, 0);
        let grid5 = generate_frame(20, 9, 5);
        // Frames should differ (agents move)
        assert_ne!(grid0, grid5);
    }

    #[test]
    fn render_logo_large_area() {
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_logo(frame, area, 3);
            })
            .unwrap();
    }
}
