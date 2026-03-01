use std::time::Duration;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::theme;
use crate::types::*;
use crate::widgets;
use tachyonfx::EffectRenderer;

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
        &mut app.tree_state,
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
        app.selection.focused_panel == FocusPanel::Radar,
    );

    let known_tmux_names = collect_tmux_names(&app.tree);
    let tracked_sessions: Vec<_> = app
        .tmux_sessions
        .iter()
        .filter(|s| known_tmux_names.contains(s.session_id.as_str()))
        .cloned()
        .collect();
    widgets::activity::render_activity_strip(frame, bottom_strip, &tracked_sessions);

    // Input prompt overlay (renders at bottom of tree panel area)
    match app.input_mode {
        InputMode::TextInput => {
            render_text_input(frame, left_panel, app);
        }
        InputMode::Confirm => {
            render_confirm(frame, left_panel, app);
        }
        InputMode::GroupPicker => {
            render_group_picker(frame, left_panel, app);
        }
        InputMode::Normal => {}
    }

    // Status message overlay
    if let Some((ref msg, _)) = app.status_message {
        let msg_width = bottom_strip.width.min(msg.len() as u16 + 4);
        let msg_area = Rect {
            x: bottom_strip.x,
            y: bottom_strip.y.saturating_sub(1),
            width: msg_width,
            height: 1,
        };
        let status = Paragraph::new(format!(" {msg} "))
            .style(Style::new().fg(theme::HAZARD).bg(theme::SURFACE));
        frame.render_widget(status, msg_area);
    }

    // Help overlay
    if app.show_help {
        render_help_overlay(frame, area);
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

// ---------------------------------------------------------------------------
// Text input prompt
// ---------------------------------------------------------------------------

fn render_text_input(frame: &mut Frame, panel_area: Rect, app: &App) {
    let label = match &app.input_context {
        Some(InputContext::NewSessionName) => "Session name",
        Some(InputContext::NewSessionCwd { .. }) => "Working directory",
        Some(InputContext::RenameSession { .. }) => "New name",
        Some(InputContext::RenameGroup { .. }) => "New name",
        Some(InputContext::NewGroupName) => "Group name",
        _ => "Input",
    };

    let prompt_height = 3u16;
    let prompt_area = Rect {
        x: panel_area.x,
        y: panel_area.y + panel_area.height.saturating_sub(prompt_height),
        width: panel_area.width,
        height: prompt_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::NEON_CYAN))
        .style(Style::new().bg(theme::SURFACE));

    let inner = block.inner(prompt_area);
    frame.render_widget(Clear, prompt_area);
    frame.render_widget(block, prompt_area);

    let cursor_char = "\u{2588}"; // █
    let content = Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::new().fg(theme::NEON_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(&app.input_buffer, Style::new().fg(theme::TEXT)),
        Span::styled(cursor_char, Style::new().fg(theme::NEON_CYAN)),
    ]);

    frame.render_widget(Paragraph::new(content), inner);
}

// ---------------------------------------------------------------------------
// Confirm dialog
// ---------------------------------------------------------------------------

fn render_confirm(frame: &mut Frame, panel_area: Rect, app: &App) {
    let message = match &app.input_context {
        Some(InputContext::ConfirmDeleteSession { .. }) => "Delete this session? (y/n)",
        Some(InputContext::ConfirmDeleteGroup { .. }) => "Delete this group? (y/n)",
        _ => "Confirm? (y/n)",
    };

    let prompt_height = 3u16;
    let prompt_area = Rect {
        x: panel_area.x,
        y: panel_area.y + panel_area.height.saturating_sub(prompt_height),
        width: panel_area.width,
        height: prompt_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::HAZARD))
        .style(Style::new().bg(theme::SURFACE));

    let inner = block.inner(prompt_area);
    frame.render_widget(Clear, prompt_area);
    frame.render_widget(block, prompt_area);

    let content = Paragraph::new(Span::styled(
        message,
        Style::new().fg(theme::HAZARD).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(content, inner);
}

// ---------------------------------------------------------------------------
// Group picker overlay
// ---------------------------------------------------------------------------

fn render_group_picker(frame: &mut Frame, panel_area: Rect, app: &App) {
    let group_count = app.picker_groups.len();
    let picker_height = (group_count as u16 + 2).min(panel_area.height); // +2 for borders

    let picker_area = Rect {
        x: panel_area.x,
        y: panel_area.y + panel_area.height.saturating_sub(picker_height),
        width: panel_area.width,
        height: picker_height,
    };

    let block = Block::default()
        .title(Span::styled(
            " Move to group ",
            Style::new().fg(theme::NEON_CYAN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::NEON_CYAN))
        .style(Style::new().bg(theme::SURFACE));

    let inner = block.inner(picker_area);
    frame.render_widget(Clear, picker_area);
    frame.render_widget(block, picker_area);

    let lines: Vec<Line> = app
        .picker_groups
        .iter()
        .enumerate()
        .map(|(i, (_gid, name))| {
            let is_selected = i == app.picker_cursor;
            let prefix = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                Style::new().fg(theme::NEON_CYAN).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(theme::TEXT)
            };
            Line::from(Span::styled(format!("{prefix}{name}"), style))
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}

// ---------------------------------------------------------------------------
// Help overlay
// ---------------------------------------------------------------------------

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let bindings: Vec<(&str, &str)> = vec![
        ("q / Q / Ctrl+C", "Quit Nexus"),
        ("Tab", "Toggle focus Tree / Radar"),
        ("?", "Toggle this help"),
        ("j / Down", "Cursor down"),
        ("k / Up", "Cursor up"),
        ("Enter (group)", "Toggle expand/collapse"),
        ("Enter (session)", "Attach or resume session"),
        ("n", "New session"),
        ("G", "New group"),
        ("r", "Rename selected item"),
        ("m", "Move session to group"),
        ("d", "Delete selected item"),
        ("x", "Kill tmux (mark detached)"),
        ("h", "Toggle past/dead sessions"),
        ("Esc", "Cancel current action"),
    ];

    let content_height = (bindings.len() as u16) + 4; // +4 for borders + title + padding
    let content_width = 50u16;

    let overlay = centered_rect(content_width, content_height, area);

    let block = Block::default()
        .title(Span::styled(
            " KEYBINDINGS ",
            Style::new().fg(theme::NEON_CYAN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::NEON_CYAN))
        .style(Style::new().bg(theme::BG));

    let inner = block.inner(overlay);
    frame.render_widget(Clear, overlay);
    frame.render_widget(block, overlay);

    let lines: Vec<Line> = bindings
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(
                    format!("{key:>18}"),
                    Style::new().fg(theme::NEON_CYAN),
                ),
                Span::styled(format!("  {desc}"), Style::new().fg(theme::TEXT)),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Collect all tmux_name values from the session tree.
fn collect_tmux_names(tree: &[TreeNode]) -> std::collections::HashSet<&str> {
    let mut names = std::collections::HashSet::new();
    for node in tree {
        match node {
            TreeNode::Session(s) => {
                if let Some(ref n) = s.tmux_name {
                    names.insert(n.as_str());
                }
            }
            TreeNode::Group(g) => {
                names.extend(collect_tmux_names(&g.children));
            }
        }
    }
    names
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}
