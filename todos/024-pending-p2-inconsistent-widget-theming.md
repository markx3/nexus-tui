---
status: pending
priority: p2
issue_id: "024"
tags: [code-review, consistency, ui]
dependencies: []
---

# Inconsistent Widget Theming — tree.rs and radar.rs Bypass Theme System

## Problem Statement

`detail.rs`, `activity.rs`, and `top_bar.rs` consistently use `theme::border_style_for()` and `theme::border_for()`. But `tree.rs` and `radar.rs` duplicate the border/focus styling logic inline with direct color constant access.

This means tree/radar use `DIM` for unfocused borders while the theme system uses `BORDER` color. Any future theme changes require updating both the centralized system AND the inline duplicates.

## Findings

- **Pattern Recognition:** Finding 4.1 (MEDIUM) — table showing 3/5 widgets use theme system, 2/5 inline.

**Affected locations:**
- `src/widgets/tree.rs:130-146` — inline border styling
- `src/widgets/radar.rs:47-64` — inline border styling

**Correct pattern (used by detail, activity, top_bar):**
```rust
.border_set(theme::border_for(PanelType::Detail))
.border_style(theme::border_style_for(PanelType::Detail, focused))
```

## Proposed Solutions

### Option A: Update tree.rs and radar.rs to use theme system (Recommended)
Replace inline border logic with `theme::border_style_for(PanelType::SessionTree, focused)`.

- **Pros:** Consistent theming, ~20 LOC reduction, single source of truth
- **Cons:** None
- **Effort:** Small
- **Risk:** Low

## Technical Details

**Affected files:** `src/widgets/tree.rs`, `src/widgets/radar.rs`

## Acceptance Criteria

- [ ] All 5 widgets use `theme::border_style_for()` and `theme::border_for()`
- [ ] No inline border color logic in any widget
- [ ] Visual appearance unchanged (verify border colors match)

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Consistency across widget rendering |
