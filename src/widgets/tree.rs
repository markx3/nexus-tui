use std::collections::HashSet;

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::time_utils;
use crate::types::{GroupIcon, GroupId, PanelType, SessionStatus, ThemeElement, TreeNode};
use crate::widgets::tree_state::{FlatNodeKind, TreeState};

// ---------------------------------------------------------------------------
// Icons
// ---------------------------------------------------------------------------

const ICON_ROOT: &str = "\u{25C8}"; // ◈
const ICON_SUBGROUP: &str = "\u{2B21}"; // ⬡
const ICON_COLLAPSED: &str = "\u{25B6}"; // ▶
const ICON_ACTIVE: &str = "\u{25CF}"; // ●
const ICON_DETACHED: &str = "\u{25CB}"; // ○
const ICON_DEAD: &str = "\u{25CC}"; // ◌
const ICON_ATTENTION: &str = "\u{25C9}"; // ◉

// ---------------------------------------------------------------------------
// Tree renderer
// ---------------------------------------------------------------------------

/// Render the session tree.
///
/// Returns `Vec<(tmux_name, Rect)>` for session rows currently needing
/// attention, so the caller can apply TachyonFX pulse effects on top.
pub fn render_tree(
    frame: &mut Frame,
    area: Rect,
    tree: &[TreeNode],
    state: &mut TreeState,
    focused: bool,
    attention: &HashSet<String>,
) -> Vec<(String, Rect)> {
    let title_style = if focused {
        theme::style_for(ThemeElement::Primary).add_modifier(Modifier::BOLD)
    } else {
        theme::style_for(ThemeElement::Dim)
    };

    let block = Block::default()
        .title(Span::styled(" SESSION TREE ", title_style))
        .borders(Borders::ALL)
        .border_set(theme::border_for(PanelType::SessionTree))
        .border_style(theme::border_style_for(PanelType::SessionTree, focused))
        .style(theme::style_for(ThemeElement::Surface));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return Vec::new();
    }

    let flat = state.visible_nodes(tree);
    if flat.is_empty() {
        let empty = Paragraph::new("No sessions. Press 'n' to create one.")
            .style(Style::new().fg(theme::dim()));
        frame.render_widget(empty, inner);
        return Vec::new();
    }

    // Precompute which collapsed groups contain attention sessions
    let attention_groups = groups_with_attention(tree, attention);

    // Ensure cursor is within viewport
    let viewport_h = inner.height as usize;
    state.ensure_cursor_visible(viewport_h);

    let start = state.scroll_offset;
    let end = (start + viewport_h).min(flat.len());

    const INDENTS: [&str; 8] = [
        "",
        "  ",
        "    ",
        "      ",
        "        ",
        "          ",
        "            ",
        "              ",
    ];

    let mut lines: Vec<Line> = Vec::with_capacity(viewport_h);
    let mut attention_rects: Vec<(String, Rect)> = Vec::new();

    // Scroll-up indicator
    if start > 0 {
        lines.push(Line::from(Span::styled(
            "  \u{25B2} more above",
            Style::new().fg(theme::dim()),
        )));
    }

    let content_start = if start > 0 { 1 } else { 0 };
    let content_end_budget = if end < flat.len() { 1 } else { 0 };
    let content_slots = viewport_h.saturating_sub(content_start + content_end_budget);

    let actual_end = (start + content_start + content_slots).min(flat.len());

    for (flat_idx, node) in flat
        .iter()
        .enumerate()
        .take(actual_end)
        .skip(start + content_start)
    {
        let indent = INDENTS
            .get(node.depth as usize)
            .unwrap_or(&INDENTS[INDENTS.len() - 1]);
        let is_selected = flat_idx == state.cursor_index;

        // Track whether this row is an attention session (for effect overlay)
        let mut attention_tmux_name: Option<String> = None;

        let line = match &node.node {
            FlatNodeKind::Group {
                id,
                icon,
                name,
                child_count,
                collapsed,
            } => {
                // Collapsed groups with attention children get hazard-colored icon
                let has_attention = *collapsed && attention_groups.contains(id);

                let icon_str = if *collapsed {
                    ICON_COLLAPSED
                } else {
                    match icon {
                        GroupIcon::Root => ICON_ROOT,
                        GroupIcon::SubGroup => ICON_SUBGROUP,
                    }
                };

                let icon_color = if has_attention {
                    theme::hazard()
                } else {
                    theme::primary()
                };
                let text_color = if is_selected {
                    theme::text()
                } else {
                    theme::dim()
                };
                let count_str = format!(" ({})", child_count);

                Line::from(vec![
                    Span::raw(*indent),
                    Span::styled(icon_str, Style::new().fg(icon_color)),
                    Span::styled(format!(" {}", name), Style::new().fg(text_color)),
                    Span::styled(count_str, Style::new().fg(theme::dim())),
                ])
            }
            FlatNodeKind::Session { summary } => {
                let is_attention = summary
                    .tmux_name
                    .as_deref()
                    .is_some_and(|n| attention.contains(n));

                // Attention rows use dim() as base — the TachyonFX pulse
                // animates fg from hazard → dim → hazard, covering the full row.
                let (icon_str, icon_color, name_color) = if is_attention {
                    (ICON_ATTENTION, theme::dim(), theme::dim())
                } else {
                    match summary.status {
                        SessionStatus::Active => {
                            (ICON_ACTIVE, theme::secondary(), theme::secondary())
                        }
                        SessionStatus::Detached => {
                            if time_utils::is_stale(&summary.last_active, 7 * 86400) {
                                (ICON_DETACHED, theme::dim(), theme::dim())
                            } else {
                                (ICON_DETACHED, theme::text(), theme::text())
                            }
                        }
                        SessionStatus::Dead => (ICON_DEAD, theme::dim(), theme::dim()),
                    }
                };

                if is_attention {
                    attention_tmux_name = summary.tmux_name.clone();
                }

                let rel_time = time_utils::relative_time(&summary.last_active);

                let mut spans = vec![
                    Span::raw(*indent),
                    Span::styled(icon_str, Style::new().fg(icon_color)),
                    Span::styled(
                        format!(" {}", summary.display_name),
                        Style::new().fg(name_color),
                    ),
                    Span::styled(format!("  {}", rel_time), Style::new().fg(theme::dim())),
                ];

                // Show status tag for non-active
                if summary.status == SessionStatus::Dead {
                    spans.push(Span::styled(
                        " [dead]",
                        Style::new().fg(theme::dim()).add_modifier(Modifier::DIM),
                    ));
                }

                Line::from(spans)
            }
        };

        // Apply selection highlight
        let line = if is_selected {
            let bg = if focused {
                theme::derive_selection_bg()
            } else {
                theme::derive_unfocused_selection_bg()
            };
            Line::from(
                line.spans
                    .into_iter()
                    .map(|s| s.patch_style(Style::new().bg(bg)))
                    .collect::<Vec<_>>(),
            )
        } else {
            line
        };

        // Record the Rect for this attention row (before pushing to lines)
        if let Some(tmux_name) = attention_tmux_name {
            let row_y = inner.y + lines.len() as u16;
            attention_rects.push((
                tmux_name,
                Rect {
                    x: inner.x,
                    y: row_y,
                    width: inner.width,
                    height: 1,
                },
            ));
        }

        lines.push(line);
    }

    // Scroll-down indicator
    if end < flat.len() {
        lines.push(Line::from(Span::styled(
            "  \u{25BC} more below",
            Style::new().fg(theme::dim()),
        )));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);

    attention_rects
}

// ---------------------------------------------------------------------------
// Attention helpers
// ---------------------------------------------------------------------------

/// Collect group IDs that contain (recursively) at least one session needing attention.
fn groups_with_attention(tree: &[TreeNode], attention: &HashSet<String>) -> HashSet<GroupId> {
    let mut result = HashSet::new();
    has_attention_inner(tree, attention, &mut result);
    result
}

fn has_attention_inner(
    nodes: &[TreeNode],
    attention: &HashSet<String>,
    result: &mut HashSet<GroupId>,
) -> bool {
    let mut found = false;
    for node in nodes {
        match node {
            TreeNode::Session(s) => {
                if s.tmux_name
                    .as_deref()
                    .is_some_and(|n| attention.contains(n))
                {
                    found = true;
                }
            }
            TreeNode::Group(g) => {
                if has_attention_inner(&g.children, attention, result) {
                    result.insert(g.id);
                    found = true;
                }
            }
        }
    }
    found
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_tree_no_panic() {
        use crate::mock;
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);
        let attention = HashSet::new();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &mut state, true, &attention);
            })
            .unwrap();
    }

    #[test]
    fn test_render_tree_unfocused_no_panic() {
        use crate::mock;
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);
        let attention = HashSet::new();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &mut state, false, &attention);
            })
            .unwrap();
    }

    #[test]
    fn test_render_tree_empty_no_panic() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree: Vec<TreeNode> = vec![];
        let mut state = TreeState::new(&tree);
        let attention = HashSet::new();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &mut state, true, &attention);
            })
            .unwrap();
    }

    #[test]
    fn test_render_tree_tiny_area_no_panic() {
        use crate::mock;
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(5, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);
        let attention = HashSet::new();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &mut state, true, &attention);
            })
            .unwrap();
    }

    #[test]
    fn test_render_tree_with_attention_no_panic() {
        use crate::mock;
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let tree = mock::mock_tree();
        let mut state = TreeState::new(&tree);
        // Add a tmux name that matches one of the mock sessions
        let mut attention = HashSet::new();
        attention.insert("mock-session".to_string());

        terminal
            .draw(|frame| {
                let area = frame.area();
                let rects = render_tree(frame, area, &tree, &mut state, true, &attention);
                // Should return attention rects (or empty if no mock session matches)
                let _ = rects;
            })
            .unwrap();
    }
}
