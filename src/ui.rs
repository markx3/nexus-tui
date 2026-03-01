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
        Block::default().style(Style::new().bg(theme::bg())),
        area,
    );

    // Terminal too small guard
    if area.width < 80 || area.height < 24 {
        let msg = Paragraph::new("Terminal too small. Minimum: 80x24")
            .style(Style::new().fg(theme::hazard()).bg(theme::bg()));
        frame.render_widget(msg, area);
        return;
    }

    // Top-level: top bar + main area (no bottom strip — activity removed)
    let [top_bar, main_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .areas(area);

    // Main area: tree (13%) + right column (fills remainder)
    let [left_panel, right_column] = Layout::horizontal([
        Constraint::Percentage(13),
        Constraint::Fill(1),
    ])
    .areas(main_area);

    // Right column: interactor fills everything
    let interactor_area = right_column;

    // Split left panel: tree + optional logo
    let (tree_area, logo_area) = if left_panel.height >= 20 {
        let [tree, logo] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(9),
        ])
        .areas(left_panel);
        (tree, Some(logo))
    } else {
        (left_panel, None)
    };

    // Render each zone
    let (session_count, active_count) = app.session_counts();
    widgets::top_bar::render_top_bar(frame, top_bar, session_count, active_count);

    widgets::tree::render_tree(
        frame,
        tree_area,
        &app.tree,
        &mut app.tree_state,
        true, // tree is always "focused" now (no focus switching)
    );

    if let Some(logo_area) = logo_area {
        widgets::logo::render_logo(frame, logo_area, app.logo_frame);
    }

    // Render the session interactor
    let interactor_content = app.interactor_state.as_ref().and_then(|is| is.current_content.as_ref());
    let interactor_session_name = app
        .interactor_state
        .as_ref()
        .and_then(|is| is.current_session_name.as_deref());
    let log_scroll = app
        .interactor_state
        .as_ref()
        .map(|is| is.log_scroll_offset)
        .unwrap_or(0);
    widgets::interactor::render_interactor(
        frame,
        interactor_area,
        interactor_content,
        interactor_session_name,
        false,
        log_scroll,
    );

    // Input prompt overlay (renders over full main area for readability)
    match app.input_mode {
        InputMode::TextInput => {
            render_text_input(frame, main_area, app);
        }
        InputMode::Confirm => {
            render_confirm(frame, main_area, app);
        }
        InputMode::GroupPicker => {
            render_group_picker(frame, main_area, app);
        }
        InputMode::Normal => {}
    }

    // Status message overlay
    if let Some((ref msg, _)) = app.status_message {
        let msg_width = area.width.min(msg.len() as u16 + 4);
        let msg_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: msg_width,
            height: 1,
        };
        let status = Paragraph::new(format!(" {msg} "))
            .style(Style::new().fg(theme::hazard()).bg(theme::surface()));
        frame.render_widget(status, msg_area);
    }

    // Help overlay
    if app.show_help {
        render_help_overlay(frame, area);
    }

    // Apply TachyonFX boot effects (skip once done)
    if !app.boot_done {
        let zones = [top_bar, left_panel, right_column];
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

    let is_cwd = matches!(app.input_context, Some(InputContext::NewSessionCwd { .. }));
    let has_suggestions = is_cwd && !app.path_suggestions.is_empty();

    const VISIBLE_MAX: usize = 5;

    let prompt_height = 3u16;
    let (visible_count, scroll_offset) = if has_suggestions {
        let total = app.path_suggestions.len();
        let vis = total.min(VISIBLE_MAX);
        let offset = if total <= VISIBLE_MAX {
            0
        } else {
            app.path_suggestion_cursor
                .saturating_sub(VISIBLE_MAX / 2)
                .min(total - VISIBLE_MAX)
        };
        (vis, offset)
    } else {
        (0, 0)
    };
    let suggestion_height = if has_suggestions {
        visible_count as u16 + 2 // +2 for borders
    } else {
        0
    };
    let total_height = prompt_height + suggestion_height;

    let prompt_area = Rect {
        x: panel_area.x,
        y: panel_area.y + panel_area.height.saturating_sub(total_height),
        width: panel_area.width,
        height: total_height,
    };

    // Render suggestion dropdown above the input prompt
    if has_suggestions {
        let suggestion_area = Rect {
            x: prompt_area.x,
            y: prompt_area.y,
            width: prompt_area.width,
            height: suggestion_height,
        };

        let total = app.path_suggestions.len();
        let can_scroll_up = scroll_offset > 0;
        let can_scroll_down = scroll_offset + visible_count < total;
        let scroll_indicator = match (can_scroll_up, can_scroll_down) {
            (true, true) => " \u{25B2}\u{25BC} ",
            (true, false) => " \u{25B2} ",
            (false, true) => " \u{25BC} ",
            (false, false) => "",
        };

        let mut suggestion_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(theme::primary()))
            .style(Style::new().bg(theme::surface()));
        if !scroll_indicator.is_empty() {
            suggestion_block = suggestion_block.title(Span::styled(
                scroll_indicator,
                Style::new().fg(theme::dim()),
            ));
        }

        let suggestion_inner = suggestion_block.inner(suggestion_area);
        frame.render_widget(Clear, suggestion_area);
        frame.render_widget(suggestion_block, suggestion_area);

        let visible_slice = &app.path_suggestions[scroll_offset..scroll_offset + visible_count];
        let lines: Vec<Line> = visible_slice
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let global_index = scroll_offset + i;
                let is_selected = global_index == app.path_suggestion_cursor;
                let prefix = if is_selected { "> " } else { "  " };
                let style = if is_selected {
                    Style::new().fg(theme::primary()).add_modifier(Modifier::BOLD)
                } else {
                    Style::new().fg(theme::text())
                };
                Line::from(Span::styled(format!("{prefix}{path}"), style))
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), suggestion_inner);
    }

    // Render the text input prompt below suggestions
    let input_area = Rect {
        x: prompt_area.x,
        y: prompt_area.y + suggestion_height,
        width: prompt_area.width,
        height: prompt_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::primary()))
        .style(Style::new().bg(theme::surface()));

    let inner = block.inner(input_area);
    frame.render_widget(Clear, input_area);
    frame.render_widget(block, input_area);

    let cursor_char = "\u{2588}"; // █
    let content = Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::new().fg(theme::primary()).add_modifier(Modifier::BOLD),
        ),
        Span::styled(&app.input_buffer, Style::new().fg(theme::text())),
        Span::styled(cursor_char, Style::new().fg(theme::primary())),
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
        .border_style(Style::new().fg(theme::hazard()))
        .style(Style::new().bg(theme::surface()));

    let inner = block.inner(prompt_area);
    frame.render_widget(Clear, prompt_area);
    frame.render_widget(block, prompt_area);

    let content = Paragraph::new(Span::styled(
        message,
        Style::new().fg(theme::hazard()).add_modifier(Modifier::BOLD),
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

    let title = if matches!(app.input_context, Some(InputContext::NewSessionGroup { .. })) {
        " Select group "
    } else {
        " Move to group "
    };
    let block = Block::default()
        .title(Span::styled(
            title,
            Style::new().fg(theme::primary()).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::primary()))
        .style(Style::new().bg(theme::surface()));

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
                Style::new().fg(theme::primary()).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(theme::text())
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
        ("", "-- Nexus Commands (Alt+key) --"),
        ("Alt+q", "Quit Nexus"),
        ("Alt+h / Alt+?", "Toggle this help"),
        ("Alt+j", "Cursor down"),
        ("Alt+k", "Cursor up"),
        ("Alt+Enter", "Toggle expand/collapse"),
        ("Alt+n", "New session"),
        ("Alt+g", "New group"),
        ("Alt+r", "Rename selected item"),
        ("Alt+m", "Move session to group"),
        ("Alt+d", "Delete selected item"),
        ("Alt+x", "Kill tmux (mark detached)"),
        ("Alt+H", "Toggle past/dead sessions"),
        ("Alt+f", "Fullscreen attach to session"),
        ("Alt+t / Alt+T", "Cycle theme"),
        ("Alt+l", "Open lazygit in session cwd"),
        ("", ""),
        ("", "All other keys are forwarded to"),
        ("", "the embedded Claude Code session."),
        ("", ""),
        ("", "macOS: Enable 'Use Option as Meta"),
        ("", "key' in Terminal/iTerm2 settings."),
    ];

    let content_height = (bindings.len() as u16) + 4;
    let content_width = 52u16;

    let overlay = centered_rect(content_width, content_height, area);

    let block = Block::default()
        .title(Span::styled(
            " KEYBINDINGS ",
            Style::new().fg(theme::primary()).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::primary()))
        .style(Style::new().bg(theme::bg()));

    let inner = block.inner(overlay);
    frame.render_widget(Clear, overlay);
    frame.render_widget(block, overlay);

    let lines: Vec<Line> = bindings
        .iter()
        .map(|(key, desc)| {
            if key.is_empty() {
                Line::from(Span::styled(
                    format!("  {desc}"),
                    Style::new().fg(theme::dim()),
                ))
            } else {
                Line::from(vec![
                    Span::styled(
                        format!("{key:>18}"),
                        Style::new().fg(theme::primary()),
                    ),
                    Span::styled(format!("  {desc}"), Style::new().fg(theme::text())),
                ])
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(lines), inner);
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
