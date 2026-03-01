use std::time::Duration;

use ratatui::layout::{Constraint, Layout};
use ratatui::style::Style;
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use tachyonfx::EffectRenderer;

use crate::app::App;
use crate::theme;
use crate::types::*;
use crate::widgets;

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

    // Render each zone with real state
    let (session_count, active_count) = app.session_counts();
    widgets::top_bar::render_top_bar(frame, top_bar, session_count, active_count);

    widgets::tree::render_tree(
        frame,
        left_panel,
        &app.tree,
        &app.tree_state,
        app.selection.focused_panel == FocusPanel::Tree,
    );

    widgets::radar::render_radar(
        frame,
        radar_area,
        &app.radar_state,
        app.selection.focused_panel == FocusPanel::Radar,
    );

    let selected_session = app.selected_session();
    widgets::detail::render_detail(
        frame,
        detail_area,
        selected_session,
        app.selection.focused_panel == FocusPanel::Radar, // detail focus follows radar
    );

    widgets::activity::render_activity_strip(frame, bottom_strip, &app.tmux_windows);

    // Status message overlay (Todo 023)
    if let Some((ref msg, _)) = app.status_message {
        let msg_width = bottom_strip.width.min(msg.len() as u16 + 4);
        let msg_area = ratatui::layout::Rect {
            x: bottom_strip.x,
            y: bottom_strip.y.saturating_sub(1),
            width: msg_width,
            height: 1,
        };
        let status = Paragraph::new(format!(" {msg} "))
            .style(Style::new().fg(theme::HAZARD).bg(theme::SURFACE));
        frame.render_widget(status, msg_area);
    }

    // Apply TachyonFX boot effects (skip once done)
    if !app.boot_done {
        let zones = [top_bar, left_panel, radar_area, detail_area, bottom_strip];
        for (effect, &zone) in app.boot_effects.iter_mut().zip(zones.iter()) {
            frame.render_effect(effect, zone, elapsed.into());
        }
        if app.boot_effects.iter().all(|e| e.done()) {
            app.boot_done = true;
            app.boot_effects.clear();
        }
    }
}
