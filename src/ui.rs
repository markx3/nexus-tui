use std::time::Duration;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tachyonfx::{fx, Effect, EffectRenderer, Motion};

use crate::app::App;
use crate::theme;

pub fn draw(frame: &mut Frame, app: &mut App, elapsed: Duration) {
    let area = frame.area();

    // Fill background
    frame.render_widget(
        Block::default().style(Style::new().bg(theme::BG)),
        area,
    );

    // Terminal too small guard
    if area.width < 80 || area.height < 24 {
        let msg = Paragraph::new("Terminal too small. Minimum: 80x24")
            .style(Style::new().fg(theme::HAZARD).bg(theme::BG));
        frame.render_widget(msg, area);
        return;
    }

    // Top-level: top bar, main area, bottom strip
    let [top_bar, main_area, bottom_strip] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(area);

    // Main area: left panel, right column
    let [left_panel, right_column] = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .areas(main_area);

    // Right column: radar (top), detail (bottom)
    let [radar_area, detail_area] = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .areas(right_column);

    // Render each zone
    draw_top_bar(frame, top_bar);
    draw_session_tree(frame, left_panel);
    draw_radar(frame, radar_area);
    draw_detail(frame, detail_area);
    draw_activity_strip(frame, bottom_strip);

    // Apply TachyonFX boot effects
    let zones = [top_bar, left_panel, radar_area, detail_area, bottom_strip];
    for (effect, &zone) in app.boot_effects.iter_mut().zip(zones.iter()) {
        frame.render_effect(effect, zone, elapsed.into());
    }
}

fn draw_top_bar(frame: &mut Frame, area: Rect) {
    let status = Line::from(vec![
        Span::styled(" SYS:", Style::new().fg(theme::DIM)),
        Span::styled("ONLINE", Style::new().fg(theme::ACID_GREEN)),
        Span::styled(" \u{2550}\u{2550} ", Style::new().fg(theme::DIM)),
        Span::styled("SESSIONS:", Style::new().fg(theme::DIM)),
        Span::styled("--", Style::new().fg(theme::TEXT)),
        Span::styled(" \u{2550}\u{2550} ", Style::new().fg(theme::DIM)),
        Span::styled("ACTIVE:", Style::new().fg(theme::DIM)),
        Span::styled("--", Style::new().fg(theme::TEXT)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::NEON_CYAN))
        .style(Style::new().bg(theme::SURFACE));

    let paragraph = Paragraph::new(status).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_session_tree(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " SESSION TREE ",
            Style::new().fg(theme::NEON_CYAN),
        ))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::NEON_CYAN))
        .style(Style::new().bg(theme::SURFACE));

    let content = Paragraph::new("No sessions loaded")
        .style(Style::new().fg(theme::DIM))
        .block(block);

    frame.render_widget(content, area);
}

fn draw_radar(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " SESSION RADAR ",
            Style::new().fg(theme::NEON_CYAN),
        ))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::DIM))
        .style(Style::new().bg(theme::SURFACE));

    let content = Paragraph::new("\u{25c9}")
        .style(Style::new().fg(theme::NEON_CYAN))
        .alignment(ratatui::layout::Alignment::Center)
        .block(block);

    frame.render_widget(content, area);
}

fn draw_detail(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " DETAIL ",
            Style::new().fg(theme::NEON_CYAN),
        ))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::DIM))
        .style(Style::new().bg(theme::SURFACE));

    let content = Paragraph::new("Select a session to view details")
        .style(Style::new().fg(theme::DIM))
        .block(block);

    frame.render_widget(content, area);
}

fn draw_activity_strip(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " ACTIVITY ",
            Style::new().fg(theme::NEON_CYAN),
        ))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::NEON_CYAN))
        .style(Style::new().bg(theme::SURFACE));

    let content = Paragraph::new("No active sessions")
        .style(Style::new().fg(theme::DIM))
        .block(block);

    frame.render_widget(content, area);
}

pub fn create_boot_effects() -> Vec<Effect> {
    let bg = theme::BG;
    vec![
        fx::sweep_in(Motion::LeftToRight, 15, 0, bg, 400u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, bg, 500u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, bg, 500u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, bg, 400u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, bg, 300u32),
    ]
}
