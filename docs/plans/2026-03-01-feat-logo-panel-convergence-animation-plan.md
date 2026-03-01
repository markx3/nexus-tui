---
title: "feat: Add animated logo panel below session tree"
type: feat
date: 2026-03-01
brainstorm: docs/brainstorms/2026-03-01-logo-panel-brainstorm.md
---

# feat: Add Animated Logo Panel Below Session Tree

## Overview

Add a fixed-height panel below the session tree in the left column that displays an
animated ASCII art "agent swarm convergence" logo. Small agent particles orbit toward
a central nexus point (◉), cycling through 10-12 hand-crafted frames at ~300ms per
frame. The panel is purely decorative — no interaction or focus state.

## Motivation

Nexus is a cyberpunk-themed TUI. The left column currently has only the session tree,
leaving dead space at the bottom on tall terminals. A subtle animated logo reinforces
the brand identity and the "agent swarm" concept central to the app.

## Proposed Solution

Static frame array approach: define all animation frames as string slice constants,
advance a frame index based on elapsed time, and render the current frame with themed
colors. This is the simplest approach and gives full artistic control.

## Layout Change

```
BEFORE                          AFTER
┌──────────────────┐            ┌──────────────────┐
│    top_bar (3)   │            │    top_bar (3)    │
├────────┬─────────┤            ├────────┬──────────┤
│        │         │            │  tree  │          │
│  tree  │  right  │            │ (Fill) │  right   │
│ (Fill) │ column  │            ├────────┤  column  │
│        │         │            │  logo  │          │
│        │         │            │  (9)   │          │
└────────┴─────────┘            └────────┴──────────┘
```

## Technical Approach

### Phase 1: Types & Theme Foundation

**Files:** `src/types.rs`, `src/theme.rs`

1. Add `Logo` variant to `PanelType` enum (`types.rs:224-230`)
2. Add `LogoAgent` and `LogoNexus` variants to `ThemeElement` enum (`types.rs:197-222`)
3. Add `style_for()` match arms in `theme.rs:29-56`:
   - `LogoAgent` → `Style::new().fg(DIM)` (subtle particles)
   - `LogoNexus` → `Style::new().fg(NEON_CYAN)` (glowing center)
4. Add `border_for(PanelType::Logo)` match arm in `theme.rs:61-68` → `PLAIN` borders
5. Update exhaustive test arrays:
   - `style_for_returns_non_default_for_all_elements` (`theme.rs:101-124`)
   - `border_for_returns_valid_sets` (`theme.rs:136-152`)

### Phase 2: Logo Widget

**Files:** `src/widgets/logo.rs` (new), `src/widgets/mod.rs`

1. Create `src/widgets/logo.rs` containing:

   - `const LOGO_FRAMES: &[&[&str]]` — 10-12 frames, each frame is a slice of
     string lines. Content width: 18 chars (for 20-col panel minus borders).
     Content height: 7 lines (for 9-row panel minus borders).

   - ASCII art design: agent particles (`∙`, `◆`, `·`) orbit around a central
     nexus symbol (`◉`). Particles shift positions across frames to create
     orbital motion. Connection lines (`─`, `│`, `┼`) link some agents to center.

   - `pub fn render_logo(frame: &mut Frame, area: Rect, frame_index: usize)`:
     - Build `Block` with title `" ◉ NEXUS "`, PLAIN borders, themed border style
     - Use `theme::border_for(PanelType::Logo)` and
       `theme::border_style_for(PanelType::Logo, false)` (never focused)
     - Background: `theme::style_for(ThemeElement::Surface)` (matches all panels)
     - Get inner area, early-return if zero-size (match `tree.rs:50` guard pattern)
     - Select frame: `LOGO_FRAMES[frame_index % LOGO_FRAMES.len()]`
     - Render each line as a `Line` of `Span`s:
       - Characters `◉` → `ThemeElement::LogoNexus` style
       - Characters `∙`, `◆`, `·`, `─`, `│`, `┼` → `ThemeElement::LogoAgent` style
       - Everything else → `ThemeElement::Dim` style (background filler)
     - Use `Paragraph::new(lines).alignment(Alignment::Center)` to center
       content horizontally — handles all terminal widths gracefully
     - Truncate/skip lines if inner area is shorter than frame content

   - Tests: `render_logo_no_panic` (normal area), `render_logo_zero_area`,
     `render_logo_all_frames` (iterate all indices to verify no out-of-bounds)

2. Add `pub mod logo;` to `src/widgets/mod.rs`

### Phase 3: App State & Timing

**Files:** `src/app.rs`

1. Add fields to `App` struct (`app.rs:24-57`):

   ```rust
   logo_frame: usize,
   logo_last_advance: Instant,
   ```

2. Initialize in `App::new()` (`app.rs:60-88`):

   ```rust
   logo_frame: 0,
   logo_last_advance: Instant::now(),
   ```

3. Add frame advance logic in `event_loop()` after the tmux poll block (~`app.rs:136`):

   ```rust
   const LOGO_FRAME_INTERVAL: Duration = Duration::from_millis(300);

   if now.duration_since(self.logo_last_advance) >= LOGO_FRAME_INTERVAL {
       self.logo_frame = self.logo_frame.wrapping_add(1);
       self.logo_last_advance = now;
   }
   ```

   This uses `wrapping_add` so it never panics on overflow; the render function
   uses modulo to wrap to the frame count.

### Phase 4: Layout & Rendering

**Files:** `src/ui.rs`

1. Split `left_panel` into tree + logo (`ui.rs` after line 44). Hide the logo
   panel when the left column is too short (< 20 rows), giving the tree full space:

   ```rust
   let show_logo = left_panel.height >= 20;
   let [tree_area, logo_area] = if show_logo {
       let areas = Layout::vertical([
           Constraint::Fill(1),
           Constraint::Length(9),
       ])
       .areas(left_panel);
       (areas[0], Some(areas[1]))
   } else {
       (left_panel, None)
   };
   ```

2. Update tree render call to use `tree_area` instead of `left_panel` (`ui.rs:57`)

3. Add logo render call after tree (conditionally):

   ```rust
   if let Some(logo_area) = logo_area {
       widgets::logo::render_logo(frame, logo_area, app.logo_frame);
   }
   ```

4. Update input prompt overlays to use `tree_area` instead of `left_panel`
   (`ui.rs:89-100`) — text input, confirm, and group picker should overlay
   the tree, not the logo.

5. Boot effects (`ui.rs:121-131`):
   - Keep 3 boot effects — apply the 2nd effect (tree sweep) to the entire
     `left_panel` rect before it is split. This covers both tree and logo
     with the same sweep-in, requires no changes to `fx_boot()`.
   - Update zones array to: `[top_bar, left_panel, right_column]` (unchanged)

## Acceptance Criteria

- [x] Logo panel renders below the session tree with a bordered frame and `◉ NEXUS` title
- [x] Animation cycles through 10-12 frames at ~300ms intervals
- [x] Agent particles render in DIM color, central nexus in NEON_CYAN
- [x] Tree panel adjusts height dynamically (Fill constraint) to accommodate logo
- [x] Input overlays (text input, confirm, group picker) overlay the tree area, not the logo
- [x] Boot sweep-in effect covers the logo panel
- [x] No panic on zero-size areas or terminal resize
- [x] All existing tests pass; new tests cover the logo widget

## Edge Cases

- **Small terminals (80x24)**: Left column = 21 rows. Since 21 >= 20, logo shows.
  Tree gets 12 rows (21 - 9). Tight but functional with scroll indicators.
- **Very short terminals (left column < 20 rows)**: Logo panel is hidden entirely,
  tree reclaims full height. Prevents unusably cramped tree.
- **Very narrow terminals**: At 80 cols, left panel is 20 cols → 18 content cols.
  Frames designed for 16 chars wide (safe margin), centered via `Alignment::Center`.
- **Wide terminals**: Extra horizontal space handled by centering — no visual issues.
- **`frame_index` overflow**: `wrapping_add(1)` on `usize` means it wraps at
  `usize::MAX` which is harmless since we always modulo by frame count.
- **Unicode width**: Frame art uses single-width Unicode characters only (no CJK
  double-width chars). `◉` is single-width in most terminal fonts.
- **Animation loops continuously**: Modular arithmetic (`%`), no stop condition.

## Files Changed Summary

| File | Change |
|------|--------|
| `src/types.rs` | Add `PanelType::Logo`, `ThemeElement::LogoAgent`, `ThemeElement::LogoNexus` |
| `src/theme.rs` | Add match arms for new variants, 4th boot effect, update tests |
| `src/widgets/logo.rs` | **NEW** — frame data + `render_logo()` + tests |
| `src/widgets/mod.rs` | Add `pub mod logo;` |
| `src/app.rs` | Add `logo_frame`, `logo_last_advance` fields + advance logic |
| `src/ui.rs` | Split left panel layout, render logo, update overlay targets, update boot zones |

## References

- Brainstorm: `docs/brainstorms/2026-03-01-logo-panel-brainstorm.md`
- Widget pattern: `src/widgets/detail.rs:12` (simplest existing widget)
- Timing pattern: `src/app.rs:131-136` (tmux poll interval)
- Layout: `src/ui.rs:39-51` (current layout structure)
- Theme enums: `src/types.rs:197-230`, `src/theme.rs:29-68`
