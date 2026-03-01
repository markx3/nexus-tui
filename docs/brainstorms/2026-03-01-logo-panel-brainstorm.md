---
date: 2026-03-01
topic: logo-panel
---

# Logo Panel — Agent Swarm Convergence Animation

## What We're Building

A fixed-height panel below the session tree in the left column, displaying an animated
ASCII art "agent swarm convergence" logo. Small agent particles (dots, diamonds) orbit
and drift toward a central nexus point (◉), cycling through 10-12 hand-crafted frames
at ~300ms per frame. The panel has a bordered frame with a title (e.g. `◉ NEXUS`).

The animation is atmospheric — agents render in DIM color, only the central nexus
symbol glows in NEON_CYAN. It acts as ambient branding, not a focal point.

## Layout Change

Current left column:
```
┌──────────────────┐
│    session tree   │
│    (full height)  │
└──────────────────┘
```

New left column:
```
┌──────────────────┐
│    session tree   │
│    (Fill)         │
├──── ◉ NEXUS ─────┤
│  ∙   ─┼─  ◆     │
│  ◆── ◉ ──∙     │
│  ∙   ─┼─  ∙     │
└──────────────────┘  ← 9 rows fixed
```

The tree gets `Constraint::Fill(1)` and the logo panel gets `Constraint::Length(9)`.

## Key Decisions

- **Animation style**: Convergence — agents orbit/drift toward central nexus point
- **Frame count**: 10-12 hand-crafted ASCII art frames (static array, not procedural)
- **Frame rate**: ~300ms per frame (medium pace, full cycle ~3-4 seconds)
- **Panel height**: Fixed 9 rows (7 content rows inside border)
- **Panel chrome**: Bordered with title, consistent with other panels
- **Color palette**: Subtle — agents in DIM, center nexus in NEON_CYAN only
- **Implementation**: Static `&[&str]` frame array, `frame_index` advances with elapsed time

## Why Static Frames (Approach A)

Considered three approaches:
1. **Static frame array** ✓ — hand-crafted frames, index lookup
2. Procedural generation — math-based positions, adaptive but harder to tune
3. Hybrid keyframes — overkill for a decoration panel

Static frames were chosen because:
- Dead simple implementation (index into array)
- Full artistic control over each frame
- Panel size is fixed, no need for adaptive positioning
- Easy to iterate on the art without touching logic

## Implementation Sketch

- New file: `src/widgets/logo.rs` — frame data + `render_logo()` function
- `src/widgets/mod.rs` — add `pub mod logo;`
- `src/ui.rs` — split `left_panel` into tree (Fill) + logo (Length(9))
- `src/app.rs` — add `logo_frame_index: usize` + `logo_last_advance: Instant`
  - Advance frame in event loop when 300ms elapsed
- `src/theme.rs` — add `PanelType::Logo` + `ThemeElement::LogoAgent` / `LogoNexus`
- `src/types.rs` — add the new enum variants

## Open Questions

- Exact ASCII art for each of the 10-12 frames (to be designed during implementation)
- Whether to add a TachyonFX boot effect for the logo panel (fade-in to match others)

## Next Steps

→ `/workflows:plan` for implementation details
