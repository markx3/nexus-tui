use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::time_utils;
use crate::types::{GroupIcon, PanelType, SessionStatus, ThemeElement, TreeNode};
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

// ---------------------------------------------------------------------------
// Tree renderer
// ---------------------------------------------------------------------------

pub fn render_tree(
    frame: &mut Frame,
    area: Rect,
    tree: &[TreeNode],
    state: &mut TreeState,
    focused: bool,
) {
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
        return;
    }

    let flat = state.visible_nodes(tree);
    if flat.is_empty() {
        let empty = Paragraph::new("No sessions. Press 'n' to create one.")
            .style(Style::new().fg(theme::dim()));
        frame.render_widget(empty, inner);
        return;
    }

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

        let line = match &node.node {
            FlatNodeKind::Group {
                icon,
                name,
                child_count,
                collapsed,
                ..
            } => {
                let icon_str = if *collapsed {
                    ICON_COLLAPSED
                } else {
                    match icon {
                        GroupIcon::Root => ICON_ROOT,
                        GroupIcon::SubGroup => ICON_SUBGROUP,
                    }
                };

                let icon_color = theme::primary();
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
                let (icon_str, icon_color, name_color) = match summary.status {
                    SessionStatus::Active => (ICON_ACTIVE, theme::secondary(), theme::secondary()),
                    SessionStatus::Detached => {
                        if time_utils::is_stale(&summary.last_active, 7 * 86400) {
                            (ICON_DETACHED, theme::dim(), theme::dim())
                        } else {
                            (ICON_DETACHED, theme::text(), theme::text())
                        }
                    }
                    SessionStatus::Dead => (ICON_DEAD, theme::dim(), theme::dim()),
                };

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
                if summary.status == SessionStatus::Detached {
                    spans.push(Span::styled(
                        " [detached]",
                        Style::new().fg(theme::dim()).add_modifier(Modifier::DIM),
                    ));
                } else if summary.status == SessionStatus::Dead {
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

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &mut state, true);
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

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &mut state, false);
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

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &mut state, true);
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

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &mut state, true);
            })
            .unwrap();
    }
}
