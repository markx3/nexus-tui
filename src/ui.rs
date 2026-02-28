use std::time::Duration;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use tachyonfx::EffectRenderer;

use crate::app::App;
use crate::mock;
use crate::theme;
use crate::types::*;
use crate::widgets;
use crate::widgets::radar_state::RadarState;
use crate::widgets::tree_state::TreeState;

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
    draw_session_tree(
        frame,
        left_panel,
        &app.tree_state,
        &app.mock_tree,
        app.selection.focused_panel == FocusPanel::Tree,
    );
    draw_radar(
        frame,
        radar_area,
        &app.radar_state,
        app.selection.focused_panel == FocusPanel::Radar,
    );
    draw_detail(frame, detail_area);
    draw_activity_strip(frame, bottom_strip);

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

fn draw_top_bar(frame: &mut Frame, area: Rect) {
    widgets::top_bar::render_top_bar(frame, area, 5, 2);
}

fn draw_session_tree(
    frame: &mut Frame,
    area: Rect,
    tree_state: &TreeState,
    tree: &[TreeNode],
    focused: bool,
) {
    widgets::tree::render_tree(frame, area, tree, tree_state, focused);
}

fn draw_radar(frame: &mut Frame, area: Rect, radar_state: &RadarState, focused: bool) {
    widgets::radar::render_radar(frame, area, radar_state, focused);
}

fn draw_detail(frame: &mut Frame, area: Rect) {
    let tree = mock::mock_tree();
    let session = find_first_session(&tree);
    widgets::detail::render_detail(frame, area, session.as_ref(), false);
}

fn find_first_session(tree: &[crate::types::TreeNode]) -> Option<SessionSummary> {
    for node in tree {
        match node {
            crate::types::TreeNode::Session(s) => return Some(s.clone()),
            crate::types::TreeNode::Group(g) => {
                if let Some(s) = find_first_session(&g.children) {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn draw_activity_strip(frame: &mut Frame, area: Rect) {
    let windows = mock::mock_tmux_windows();
    widgets::activity::render_activity_strip(frame, area, &windows);
}
