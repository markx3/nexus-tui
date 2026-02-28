use std::f64::consts::PI;

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::Span;
use ratatui::widgets::canvas::{Canvas, Circle, Line as CanvasLine, Points};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::theme;
use crate::widgets::radar_state::RadarState;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Canvas coordinate range: center at (0,0), extends [-50, 50].
const RANGE: f64 = 50.0;

/// Ring radii as fractions of the max range.
const RING_FRACTIONS: [f64; 4] = [0.25, 0.50, 0.75, 1.0];

/// Ring labels.
const RING_LABELS: [&str; 4] = ["today", "week", "month", "older"];

/// Number of points in the sweep arm.
const SWEEP_POINTS: usize = 40;

/// Sweep tail fade: number of trailing arms at decreasing opacity.
const SWEEP_TAIL_COUNT: usize = 6;

/// Degrees between each tail segment.
const SWEEP_TAIL_STEP: f64 = 3.0 * PI / 180.0;

// ---------------------------------------------------------------------------
// Radar renderer
// ---------------------------------------------------------------------------

/// Render the session radar widget.
pub fn render_radar(
    frame: &mut Frame,
    area: Rect,
    state: &RadarState,
    focused: bool,
) {
    let border_color = if focused {
        theme::NEON_CYAN
    } else {
        theme::DIM
    };

    let title_style = if focused {
        Style::new()
            .fg(theme::NEON_CYAN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(theme::DIM)
    };

    let block = Block::default()
        .title(Span::styled(" SESSION RADAR ", title_style))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(border_color))
        .style(Style::new().bg(theme::SURFACE));

    let canvas = Canvas::default()
        .block(block)
        .marker(Marker::Braille)
        .x_bounds([-RANGE, RANGE])
        .y_bounds([-RANGE, RANGE])
        .paint(|ctx| {
            // ---------------------------------------------------------------
            // 1. Concentric range rings
            // ---------------------------------------------------------------
            for (i, &frac) in RING_FRACTIONS.iter().enumerate() {
                let r = RANGE * frac * 0.94; // slight inset from edge
                let ring_color = if i == RING_FRACTIONS.len() - 1 {
                    // Outermost ring slightly brighter
                    ratatui::style::Color::Rgb(50, 54, 75)
                } else {
                    ratatui::style::Color::Rgb(35, 38, 55)
                };

                ctx.draw(&Circle {
                    x: 0.0,
                    y: 0.0,
                    radius: r,
                    color: ring_color,
                });

                // Ring label at the top of each ring
                let label_y = r - 1.5;
                ctx.print(
                    -2.0,
                    label_y,
                    Span::styled(
                        RING_LABELS[i],
                        Style::new().fg(ratatui::style::Color::Rgb(60, 64, 85)),
                    ),
                );
            }

            // ---------------------------------------------------------------
            // 2. Cross-hair lines (subtle axes)
            // ---------------------------------------------------------------
            let axis_color = ratatui::style::Color::Rgb(30, 33, 48);
            let outer_r = RANGE * 0.94;

            ctx.draw(&CanvasLine {
                x1: -outer_r,
                y1: 0.0,
                x2: outer_r,
                y2: 0.0,
                color: axis_color,
            });
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: -outer_r,
                x2: 0.0,
                y2: outer_r,
                color: axis_color,
            });

            // ---------------------------------------------------------------
            // 3. Center dot
            // ---------------------------------------------------------------
            ctx.draw(&Points {
                coords: &[(0.0, 0.0)],
                color: theme::NEON_CYAN,
            });

            // ---------------------------------------------------------------
            // 4. Session blips
            // ---------------------------------------------------------------
            for (i, blip) in state.blips.iter().enumerate() {
                let is_cursor = state.cursor_blip == Some(i);
                let blip_color = if blip.is_active {
                    theme::ACID_GREEN
                } else {
                    theme::DIM
                };

                ctx.draw(&Points {
                    coords: &[(blip.x, blip.y)],
                    color: blip_color,
                });

                // Selected blip: draw a highlight ring
                if is_cursor {
                    ctx.draw(&Circle {
                        x: blip.x,
                        y: blip.y,
                        radius: 3.0,
                        color: theme::NEON_CYAN,
                    });

                    // Show session name near the blip
                    let label_x = blip.x + 4.0;
                    let label_y = blip.y + 2.0;
                    let label = if blip.group_name.len() > 12 {
                        &blip.group_name[..12]
                    } else {
                        &blip.group_name
                    };
                    ctx.print(
                        label_x,
                        label_y,
                        Span::styled(
                            label.to_string(),
                            Style::new().fg(theme::NEON_CYAN),
                        ),
                    );
                }
            }

            // ---------------------------------------------------------------
            // 5. Sweep arm with fading tail
            // ---------------------------------------------------------------
            for tail in 0..SWEEP_TAIL_COUNT {
                let angle = state.sweep_angle - (tail as f64) * SWEEP_TAIL_STEP;

                // Fade factor: 1.0 for the head, decreasing for tail
                let fade = 1.0 - (tail as f64 / SWEEP_TAIL_COUNT as f64);

                let sweep_color = if tail == 0 {
                    theme::NEON_CYAN
                } else {
                    let intensity = (fade * 180.0) as u8;
                    ratatui::style::Color::Rgb(0, intensity, intensity)
                };

                // Draw sweep as a series of points along the line from center
                let sweep_r = RANGE * 0.94;
                let mut sweep_pts: Vec<(f64, f64)> = Vec::with_capacity(SWEEP_POINTS);
                for p in 0..SWEEP_POINTS {
                    let t = p as f64 / SWEEP_POINTS as f64;
                    let r = t * sweep_r;
                    sweep_pts.push((r * angle.cos(), r * angle.sin()));
                }

                ctx.draw(&Points {
                    coords: &sweep_pts,
                    color: sweep_color,
                });
            }
        });

    frame.render_widget(canvas, area);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;
    use crate::widgets::radar_state::RadarState;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_render_radar_no_panic() {
        let backend = TestBackend::new(60, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let tree = mock::mock_tree();
        let mut state = RadarState::new();
        state.compute_blips(&tree);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_radar(frame, area, &state, true);
            })
            .unwrap();
    }

    #[test]
    fn test_render_radar_unfocused_no_panic() {
        let backend = TestBackend::new(60, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let tree = mock::mock_tree();
        let mut state = RadarState::new();
        state.compute_blips(&tree);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_radar(frame, area, &state, false);
            })
            .unwrap();
    }

    #[test]
    fn test_render_radar_empty_no_panic() {
        let backend = TestBackend::new(60, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let state = RadarState::new();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_radar(frame, area, &state, true);
            })
            .unwrap();
    }

    #[test]
    fn test_render_radar_with_cursor_no_panic() {
        let backend = TestBackend::new(60, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let tree = mock::mock_tree();
        let mut state = RadarState::new();
        state.compute_blips(&tree);
        state.move_cursor_next(); // select first blip

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_radar(frame, area, &state, true);
            })
            .unwrap();
    }

    #[test]
    fn test_render_radar_small_area_no_panic() {
        let backend = TestBackend::new(10, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        let tree = mock::mock_tree();
        let mut state = RadarState::new();
        state.compute_blips(&tree);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_radar(frame, area, &state, true);
            })
            .unwrap();
    }

    #[test]
    fn test_render_radar_sweep_advanced() {
        let backend = TestBackend::new(60, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let tree = mock::mock_tree();
        let mut state = RadarState::new();
        state.compute_blips(&tree);
        state.advance_sweep(30.0); // half rotation

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_radar(frame, area, &state, true);
            })
            .unwrap();
    }
}
