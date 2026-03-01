---
status: pending
priority: p3
issue_id: "028"
tags: [code-review, testing]
dependencies: []
---

# Add Tests for app.rs — Central Coordinator Has Zero Coverage

## Problem Statement

`app.rs` is the central coordinator handling key events, tree actions, tmux polling, and selection sync. It has **zero tests**. Untested logic includes:

- `handle_key()` — global key dispatch
- `handle_tree_action()` — action processing (has a `_ => {}` catch-all hiding bugs)
- `try_launch_session()` — tmux session identity mismatch (Finding #013)
- `sanitize_tmux_name()` — truncation logic
- `find_session_cwd()`, `find_session_in_tree()` — tree traversals
- `count_sessions()`, `mark_active_sessions()` — tree walkers
- Selection sync between tree and radar

## Findings

- **Pattern Recognition:** Finding 6.4 — "the central coordinator has zero tests."

## Proposed Solutions

### Option A: Unit test pure functions + integration test key handling
1. Test `sanitize_tmux_name()` with edge cases (multi-byte, short, collisions)
2. Test `count_sessions()` and `mark_active_sessions()` with mock trees
3. Test `find_session_cwd()` and `find_session_in_tree()` with nested groups
4. Test `handle_tree_action()` to verify all actions are handled (not dropped)

- **Effort:** Medium
- **Risk:** Low

## Technical Details

**Affected files:** `src/app.rs`

## Acceptance Criteria

- [ ] `sanitize_tmux_name` tested with ASCII, multi-byte, short inputs
- [ ] Tree traversal functions tested with nested groups
- [ ] `handle_tree_action` tested to verify no silently dropped actions
- [ ] `mark_active_sessions` tested with matching/non-matching windows

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Central coordinators need test coverage |
