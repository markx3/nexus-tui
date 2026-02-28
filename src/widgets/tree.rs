use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::time_utils;
use crate::types::{GroupIcon, PanelType, ThemeElement, TreeNode};
use crate::widgets::tree_state::{FlatNodeKind, TreeState};

// ---------------------------------------------------------------------------
// Icons
// ---------------------------------------------------------------------------

const ICON_ROOT: &str = "\u{25C8}";     // ◈
const ICON_SUBGROUP: &str = "\u{2B21}"; // ⬡
const ICON_COLLAPSED: &str = "\u{25B6}"; // ▶
const ICON_ACTIVE: &str = "\u{25CF}";   // ●
const ICON_INACTIVE: &str = "\u{25CB}"; // ○

// ---------------------------------------------------------------------------
// Tree renderer
// ---------------------------------------------------------------------------

/// Render the session tree widget.
pub fn render_tree(
    frame: &mut Frame,
    area: Rect,
    tree: &[TreeNode],
    state: &TreeState,
    focused: bool,
) {
    let title_style = if focused {
        theme::style_for(ThemeElement::NeonCyan).add_modifier(Modifier::BOLD)
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
        let empty = Paragraph::new("No sessions loaded")
            .style(Style::new().fg(theme::DIM));
        frame.render_widget(empty, inner);
        return;
    }

    // Determine visible range based on scroll offset
    let viewport_h = inner.height as usize;
    let start = state.scroll_offset;
    let end = (start + viewport_h).min(flat.len());

    // Pre-computed indent strings to avoid per-frame allocations (Todo 026)
    const INDENTS: [&str; 8] = [
        "", "  ", "    ", "      ", "        ", "          ", "            ", "              ",
    ];

    let mut lines: Vec<Line> = Vec::with_capacity(viewport_h);

    for (vis_idx, flat_idx) in (start..end).enumerate() {
        let node = &flat[flat_idx];
        let indent = INDENTS.get(node.depth as usize).unwrap_or(&INDENTS[INDENTS.len() - 1]);
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

                let icon_color = theme::NEON_CYAN;

                let text_color = if is_selected {
                    theme::TEXT
                } else {
                    theme::DIM
                };

                let count_str = format!(" ({})", child_count);

                Line::from(vec![
                    Span::raw(*indent),
                    Span::styled(icon_str, Style::new().fg(icon_color)),
                    Span::styled(format!(" {}", name), Style::new().fg(text_color)),
                    Span::styled(count_str, Style::new().fg(theme::DIM)),
                ])
            }
            FlatNodeKind::Session { summary } => {
                let icon_str = if summary.is_active {
                    ICON_ACTIVE
                } else {
                    ICON_INACTIVE
                };

                let (icon_color, name_color) = if summary.is_active {
                    (theme::ACID_GREEN, theme::ACID_GREEN)
                } else if time_utils::is_stale(&summary.last_active, 7 * 86400) {
                    (theme::DIM, theme::DIM)
                } else {
                    (theme::TEXT, theme::TEXT)
                };

                let rel_time = time_utils::relative_time(&summary.last_active);
                let time_color = theme::DIM;

                Line::from(vec![
                    Span::raw(*indent),
                    Span::styled(icon_str, Style::new().fg(icon_color)),
                    Span::styled(
                        format!(" {}", summary.display_name),
                        Style::new().fg(name_color),
                    ),
                    Span::styled(
                        format!("  {}", rel_time),
                        Style::new().fg(time_color),
                    ),
                ])
            }
        };

        // Apply selection highlight
        let line = if is_selected {
            let bg = if focused {
                ratatui::style::Color::Rgb(30, 35, 60)
            } else {
                ratatui::style::Color::Rgb(25, 28, 45)
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

        // Early exit if we've filled the viewport
        if vis_idx + 1 >= viewport_h {
            break;
        }
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
        let state = TreeState::new(&tree);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &state, true);
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
        let state = TreeState::new(&tree);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &state, false);
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
        let state = TreeState::new(&tree);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &state, true);
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
        let state = TreeState::new(&tree);

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_tree(frame, area, &tree, &state, true);
            })
            .unwrap();
    }

}
