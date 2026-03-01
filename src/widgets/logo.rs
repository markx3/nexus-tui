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
// Explosion symbols: ✦ (impact) → ✧ (fading) → debris scattered around
// ---------------------------------------------------------------------------

const EXPLOSION_IMPACT: char = '✦';
const EXPLOSION_FADE: char = '✧';
const DEBRIS: char = '∗';

// Debris offsets around a collision point (dx, dy)
const DEBRIS_OFFSETS: &[(isize, isize)] = &[
    (-1, -1), (0, -1), (1, -1),
    (-2,  0),          (2,  0),
    (-1,  1), (0,  1), (1,  1),
];

// ---------------------------------------------------------------------------
// Procedural frame generation
// ---------------------------------------------------------------------------

/// Compute agent grid positions for a given frame.
fn agent_positions(width: usize, height: usize, frame_index: usize) -> Vec<(usize, usize)> {
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let scale_x = (width as f64 / 2.0) - 1.0;
    let scale_y = (height as f64 / 2.0) - 0.5;
    let t = frame_index as f64;

    AGENTS
        .iter()
        .filter_map(|agent| {
            let angle = agent.base_angle + t * agent.speed;
            let x = cx + agent.radius * scale_x * angle.cos();
            let y = cy + agent.radius * scale_y * angle.sin();
            let xi = x.round() as isize;
            let yi = y.round() as isize;
            if xi >= 0 && yi >= 0 && (xi as usize) < width && (yi as usize) < height {
                Some((xi as usize, yi as usize))
            } else {
                None
            }
        })
        .collect()
}

/// Find cells where 2+ agents overlap.
fn find_collisions(positions: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut collisions = Vec::new();
    for (i, &a) in positions.iter().enumerate() {
        for &b in &positions[i + 1..] {
            if a == b && !collisions.contains(&a) {
                collisions.push(a);
            }
        }
    }
    collisions
}

fn is_crosshair(x: usize, y: usize, cxi: usize, cyi: usize) -> bool {
    (x == cxi && y == cyi)                                         // center ◉
        || (y == cyi && x >= cxi.saturating_sub(2) && x <= cxi + 2) // horizontal arm
        || (x == cxi && y >= cyi.saturating_sub(1) && y <= cyi + 1) // vertical arm
        || (y == cyi.wrapping_sub(1) && (x == cxi.wrapping_sub(1) || x == cxi + 1))
        || (y == cyi + 1 && (x == cxi.wrapping_sub(1) || x == cxi + 1))
}

fn generate_frame(width: usize, height: usize, frame_index: usize) -> Vec<Vec<char>> {
    let mut grid = vec![vec![' '; width]; height];

    if width < 5 || height < 3 {
        return grid;
    }

    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let cxi = cx as usize;
    let cyi = cy as usize;

    // Place crosshair at center
    if cyi < height && cxi < width {
        grid[cyi][cxi] = '◉';
    }
    if cyi < height && cxi >= 2 && cxi + 2 < width {
        grid[cyi][cxi - 2] = '─';
        grid[cyi][cxi - 1] = '─';
        grid[cyi][cxi + 1] = '─';
        grid[cyi][cxi + 2] = '─';
    }
    if cxi < width && cyi >= 1 && cyi + 1 < height {
        grid[cyi - 1][cxi] = '┼';
        grid[cyi + 1][cxi] = '┼';
    }
    if cxi >= 1 && cxi + 1 < width && cyi >= 1 && cyi + 1 < height {
        grid[cyi - 1][cxi - 1] = '─';
        grid[cyi - 1][cxi + 1] = '─';
        grid[cyi + 1][cxi - 1] = '─';
        grid[cyi + 1][cxi + 1] = '─';
    }

    // Compute current agent positions and collisions
    let positions = agent_positions(width, height, frame_index);
    let current_collisions = find_collisions(&positions);

    // Check recent frames for fading explosions (look back 1-3 frames)
    let past_collisions: Vec<(usize, usize, usize)> = (1..=3_usize)
        .filter(|&age| frame_index >= age)
        .flat_map(|age| {
            let past_pos = agent_positions(width, height, frame_index - age);
            find_collisions(&past_pos)
                .into_iter()
                .map(move |(x, y)| (x, y, age))
        })
        .collect();

    // Place fading debris from past collisions (oldest first so newer overwrites)
    for &(col_x, col_y, age) in past_collisions.iter().rev() {
        match age {
            1 => {
                // Fading explosion at collision point
                if !is_crosshair(col_x, col_y, cxi, cyi) {
                    grid[col_y][col_x] = EXPLOSION_FADE;
                }
                // Scatter debris around
                for &(dx, dy) in DEBRIS_OFFSETS {
                    let nx = col_x as isize + dx;
                    let ny = col_y as isize + dy;
                    if nx >= 0 && ny >= 0 {
                        let (nx, ny) = (nx as usize, ny as usize);
                        if nx < width && ny < height && grid[ny][nx] == ' ' {
                            grid[ny][nx] = DEBRIS;
                        }
                    }
                }
            }
            2 => {
                // Sparse fading debris
                for (i, &(dx, dy)) in DEBRIS_OFFSETS.iter().enumerate() {
                    if i % 2 != 0 { continue; }
                    let nx = col_x as isize + dx;
                    let ny = col_y as isize + dy;
                    if nx >= 0 && ny >= 0 {
                        let (nx, ny) = (nx as usize, ny as usize);
                        if nx < width && ny < height && grid[ny][nx] == ' ' {
                            grid[ny][nx] = '·';
                        }
                    }
                }
            }
            _ => {} // age 3: fully faded
        }
    }

    // Place current collision impacts
    for &(col_x, col_y) in &current_collisions {
        if !is_crosshair(col_x, col_y, cxi, cyi) {
            grid[col_y][col_x] = EXPLOSION_IMPACT;
        }
    }

    // Place non-colliding agents
    let collision_set: Vec<(usize, usize)> = current_collisions.clone();
    for (i, &(x, y)) in positions.iter().enumerate() {
        if collision_set.contains(&(x, y)) {
            continue;
        }
        if grid[y][x] == ' ' {
            grid[y][x] = AGENTS[i % AGENTS.len()].symbol;
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
                    '✦' => Span::styled(ch.to_string(), nexus_style),
                    '✧' | '∗' => Span::styled(ch.to_string(), agent_style),
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
    fn find_collisions_detects_overlap() {
        let positions = vec![(5, 3), (8, 2), (5, 3), (1, 1)];
        let collisions = find_collisions(&positions);
        assert_eq!(collisions, vec![(5, 3)]);
    }

    #[test]
    fn explosion_chars_appear_on_collision() {
        // Brute-force: scan many frames for a collision
        let (w, h) = (20, 9);
        let mut found_impact = false;
        for fi in 0..200 {
            let grid = generate_frame(w, h, fi);
            for row in &grid {
                if row.contains(&EXPLOSION_IMPACT) {
                    found_impact = true;
                    break;
                }
            }
            if found_impact { break; }
        }
        assert!(found_impact, "Expected at least one collision impact in 200 frames");
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
