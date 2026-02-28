use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::types::{GroupIcon, TreeNode};
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
// Relative time formatting
// ---------------------------------------------------------------------------

/// Parse an ISO 8601 timestamp and return seconds since that time.
/// Returns `None` if the timestamp can't be parsed.
fn parse_seconds_ago(iso_ts: &str) -> Option<i64> {
    // Parse ISO 8601 format: "2026-02-28T15:30:00Z"
    // We do manual parsing to avoid pulling in chrono as a dependency.
    let ts = iso_ts.trim();
    if ts.len() < 19 {
        return None;
    }

    let year: i64 = ts.get(0..4)?.parse().ok()?;
    let month: i64 = ts.get(5..7)?.parse().ok()?;
    let day: i64 = ts.get(8..10)?.parse().ok()?;
    let hour: i64 = ts.get(11..13)?.parse().ok()?;
    let min: i64 = ts.get(14..16)?.parse().ok()?;
    let sec: i64 = ts.get(17..19)?.parse().ok()?;

    let ts_epoch = simple_utc_to_epoch(year, month, day, hour, min, sec);

    // Get current time as epoch seconds
    let now_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;

    Some(now_epoch - ts_epoch)
}

/// Convert a UTC date/time to a Unix epoch timestamp.
/// This is a simplified calculation that works for dates from 1970 onwards.
fn simple_utc_to_epoch(year: i64, month: i64, day: i64, hour: i64, min: i64, sec: i64) -> i64 {
    // Days from 1970-01-01 to the start of the given year
    let mut days: i64 = 0;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Days from start of year to start of month
    let days_in_months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += days_in_months[(m - 1) as usize];
        if m == 2 && is_leap_year(year) {
            days += 1;
        }
    }

    // Add day of month (1-indexed)
    days += day - 1;

    days * 86400 + hour * 3600 + min * 60 + sec
}

fn is_leap_year(y: i64) -> bool {
    (y.rem_euclid(4) == 0 && y.rem_euclid(100) != 0) || y.rem_euclid(400) == 0
}

/// Format seconds-ago into a human-readable relative time string.
pub fn relative_time(iso_ts: &str) -> String {
    let Some(secs) = parse_seconds_ago(iso_ts) else {
        return "?".to_string();
    };

    if secs < 0 {
        return "future".to_string();
    }

    let minutes = secs / 60;
    let hours = secs / 3600;
    let days = secs / 86400;
    let months = days / 30;

    if minutes < 1 {
        "just now".to_string()
    } else if hours < 1 {
        format!("{}m", minutes)
    } else if days < 1 {
        format!("{}h", hours)
    } else if days < 30 {
        format!("{}d", days)
    } else if months < 12 {
        format!("{}mo", months)
    } else {
        format!("{}y", days / 365)
    }
}

/// Check if a timestamp is stale (more than 7 days old).
fn is_stale(iso_ts: &str) -> bool {
    parse_seconds_ago(iso_ts)
        .map(|secs| secs > 7 * 86400)
        .unwrap_or(false)
}

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
    let border_color = if focused {
        theme::NEON_CYAN
    } else {
        theme::DIM
    };

    let title_style = if focused {
        Style::new().fg(theme::NEON_CYAN).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(theme::DIM)
    };

    let block = Block::default()
        .title(Span::styled(" SESSION TREE ", title_style))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(border_color))
        .style(Style::new().bg(theme::SURFACE));

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

    let mut lines: Vec<Line> = Vec::with_capacity(viewport_h);

    for (vis_idx, flat_idx) in (start..end).enumerate() {
        let node = &flat[flat_idx];
        let indent = "  ".repeat(node.depth as usize);
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
                    Span::raw(indent),
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
                } else if is_stale(&summary.last_active) {
                    (theme::DIM, theme::DIM)
                } else {
                    (theme::TEXT, theme::TEXT)
                };

                let rel_time = relative_time(&summary.last_active);
                let time_color = theme::DIM;

                Line::from(vec![
                    Span::raw(indent),
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
    fn test_relative_time_just_now() {
        // Use a timestamp that is very close to "now" -- we generate one on the fly
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Build an ISO timestamp from epoch
        let ts = epoch_to_iso(now as i64);
        assert_eq!(relative_time(&ts), "just now");
    }

    #[test]
    fn test_relative_time_minutes() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let ts = epoch_to_iso(now - 300); // 5 minutes ago
        assert_eq!(relative_time(&ts), "5m");
    }

    #[test]
    fn test_relative_time_hours() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let ts = epoch_to_iso(now - 7200); // 2 hours ago
        assert_eq!(relative_time(&ts), "2h");
    }

    #[test]
    fn test_relative_time_days() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let ts = epoch_to_iso(now - 86400 * 3); // 3 days ago
        assert_eq!(relative_time(&ts), "3d");
    }

    #[test]
    fn test_is_stale_recent() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let ts = epoch_to_iso(now - 86400); // 1 day ago
        assert!(!is_stale(&ts));
    }

    #[test]
    fn test_is_stale_old() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let ts = epoch_to_iso(now - 86400 * 10); // 10 days ago
        assert!(is_stale(&ts));
    }

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

    /// Helper: convert epoch seconds to ISO 8601 UTC string.
    fn epoch_to_iso(epoch: i64) -> String {
        let secs_per_day = 86400i64;
        let mut remaining = epoch;

        let mut year = 1970i64;
        loop {
            let days_in_year = if super::is_leap_year(year) { 366 } else { 365 };
            let secs_in_year = days_in_year * secs_per_day;
            if remaining < secs_in_year {
                break;
            }
            remaining -= secs_in_year;
            year += 1;
        }

        let days_in_months = if super::is_leap_year(year) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        let mut month = 1;
        for &dim in &days_in_months {
            let secs_in_month = dim * secs_per_day;
            if remaining < secs_in_month {
                break;
            }
            remaining -= secs_in_month;
            month += 1;
        }

        let day = remaining / secs_per_day + 1;
        remaining %= secs_per_day;
        let hour = remaining / 3600;
        remaining %= 3600;
        let min = remaining / 60;
        let sec = remaining % 60;

        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hour, min, sec
        )
    }
}
