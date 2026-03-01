---
status: pending
priority: p1
issue_id: "015"
tags: [code-review, performance]
dependencies: []
---

# Cache Per-Frame Tree Walks: session_counts() and selected_session()

## Problem Statement

Two functions perform full recursive tree traversals on every frame at 60 FPS:

1. **`session_counts()`** (app.rs:237) — walks entire tree to count total/active sessions for the top bar. Value only changes on tmux poll (every 2s) or tree mutation.
2. **`selected_session()`** (app.rs:228) — does O(N) recursive `find_session_in_tree()` by string comparison every frame for the detail panel. Value only changes on cursor movement.

At 60 FPS with 200 sessions, that's 120 full tree traversals per second for values that change at most once every 2 seconds.

## Findings

- **Performance Oracle:** CRITICAL-2 and CRITICAL-3 (both P0) — "60 full tree traversals per second for a value that only changes when tmux is polled."

**Affected locations:**
- `src/app.rs:237-239` — `session_counts()` called from `ui::draw`
- `src/app.rs:228-234` — `selected_session()` called from `ui::draw`
- `src/app.rs:278-297` — `count_sessions()` recursive walk
- `src/app.rs:260-276` — `find_session_in_tree()` recursive walk

## Proposed Solutions

### Option A: Cache both values in App (Recommended)
Add `cached_counts: (usize, usize)` and `cached_selected: Option<SessionSummary>` to App. Update on tmux poll, tree mutation, and cursor movement respectively.

- **Pros:** Eliminates 120 tree walks/sec, trivial implementation
- **Cons:** Must update caches at the right points
- **Effort:** Small
- **Risk:** Low

### Option B: Session index HashMap
Build `HashMap<SessionId, &SessionSummary>` at tree construction time. O(1) lookup by ID.

- **Pros:** Also speeds up `find_session_cwd()`, general-purpose
- **Cons:** Must rebuild on tree changes, more memory
- **Effort:** Medium
- **Risk:** Low

## Recommended Action

Option A for immediate fix. Option B as a follow-up if more lookups are needed.

## Technical Details

**Affected files:** `src/app.rs`, `src/ui.rs`
**Components:** App state, UI rendering

## Acceptance Criteria

- [ ] `session_counts()` returns cached value, not a tree walk
- [ ] `selected_session()` returns cached value, not a tree search
- [ ] Caches update on: tmux poll, tree reload, cursor change
- [ ] No visible behavior change in top bar or detail panel

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Per-frame caching is low-effort, high-impact |
