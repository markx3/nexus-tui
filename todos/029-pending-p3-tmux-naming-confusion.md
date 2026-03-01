---
status: pending
priority: p3
issue_id: "029"
tags: [code-review, naming, clarity]
dependencies: []
---

# Tmux Naming Confusion — kill_window/list_windows Operate on Sessions

## Problem Statement

`TmuxManager` methods use "window" naming but actually operate on tmux sessions:
- `kill_window()` runs `tmux kill-session`
- `list_windows()` runs `tmux list-sessions`
- `TmuxWindowInfo` type represents a tmux session, not a window

Tmux distinguishes sessions from windows. This naming creates confusion when reading the code or diagnosing tmux issues.

## Findings

- **Pattern Recognition:** Finding 3.2.1 — "the method kills a session, not a window."

## Proposed Solutions

### Option A: Rename to session terminology (Recommended)
- `kill_window()` → `kill_session()`
- `list_windows()` → `list_sessions()`
- `TmuxWindowInfo` → `TmuxSessionInfo`
- `tmux_windows` field in App → `tmux_sessions`

- **Effort:** Small (rename + find/replace)
- **Risk:** Low

## Technical Details

**Affected files:** `src/tmux.rs`, `src/app.rs`, `src/types.rs`, `src/ui.rs`, `src/widgets/activity.rs`

## Acceptance Criteria

- [ ] All tmux-related naming uses "session" terminology consistently
- [ ] No "window" references for tmux session operations
- [ ] All tests pass after rename

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Match domain terminology to reduce confusion |
