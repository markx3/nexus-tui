---
task: "08"
title: Cyberpunk Theme & Animations
type: feat
date: 2026-02-28
status: pending
depends_on: []
---

# 08 — Cyberpunk Theme & Animations

## Goal

A reusable theme module that defines the entire visual identity of Nexus — colors, borders, Unicode decorators, and TachyonFX animation presets.

## Scope

### Color Palette (constants module)

```
BG:      #0B0C10  (blue-black void)
SURFACE: #141726  (elevated panels)
BORDER:  #1C2333  (subtle outlines)
TEXT:    #C8D3F5  (cool blue-white)
NEON_C:  #00E5FF  (electric cyan)
NEON_M:  #FF00FF  (hot magenta)
ACID:    #39FF14  (acid green)
HAZARD:  #F7FF4A  (acid yellow)
DIM:     #4A4E69  (muted purple-gray)
```

### Border Styles

- Structural frame: heavy (`━━━ ┏ ┓ ┗ ┛`)
- Holographic/floating panels: dashed (`╌╌╌`)
- Active panel: neon cyan heavy
- Focused input: neon magenta

### Unicode Decorator Set

- `◉` radar center, `◈` group, `⬡` sub-group, `▶` active, `⊕` target
- Separator: `══` between status metrics

### TachyonFX Animation Presets

- **Boot sequence**: staggered `sweep_in` left-to-right per panel (~150ms delay between panels)
- **Panel transition**: `coalesce` (text materializes from noise, ~500ms) on session selection change
- **Border pulse**: slow `hsl_shift_fg` ping-pong on active panel borders (~2s cycle)
- **Radar sweep**: manual tick-based rotation (not TachyonFX — pure math in render loop)
- **Glitch alert**: brief `hsl_shift` to red + `dissolve` + `coalesce` (~400ms) on errors
- **Evolve text**: characters cycle through symbols before settling (~600ms) on new data fields

### Theme API

- `theme::style_for(element: ThemeElement) -> Style` — centralized style lookup
- `theme::border_for(panel: PanelType) -> BorderSet` — border style by panel type
- `theme::fx_boot() -> Effect`, `theme::fx_transition() -> Effect`, etc.

## Acceptance Criteria

- [ ] All color constants defined and accessible
- [ ] Border styles for structural, holographic, active, and focused panels
- [ ] TachyonFX effects compile and render correctly in isolation
- [ ] Boot sequence animates panels appearing sequentially
- [ ] Border pulse is visible and smooth
- [ ] Theme can be applied to any Ratatui widget via the style API
- [ ] Optional: TOML theme overrides load and apply

## Notes

This is a pure styling module — no business logic. Can be demonstrated in a standalone test harness that renders sample panels with all effects.
