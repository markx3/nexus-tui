---
status: pending
priority: p1
issue_id: "014"
tags: [code-review, performance]
dependencies: []
---

# Cache visible_nodes() — Called 3-4x Per Key Event With Full Tree Clone

## Problem Statement

`TreeState::visible_nodes()` allocates a new `Vec<FlatNode>` every call, cloning the entire `SessionSummary` (13 fields including multiple Strings) for each session node. This function is called:

1. `move_cursor_down()` — just to get the count
2. `move_cursor_up()` — just to get the count
3. `selected_target()` — to find node at cursor
4. `render_tree()` — the actual render

A single arrow-key press triggers 2-3 full tree flattenings. At 100 sessions, that's ~300 `SessionSummary` clones per keystroke plus ~100 per frame at 60 FPS.

## Findings

- **Performance Oracle:** CRITICAL-1 (P0) — "the most impactful performance issue."
- **Architecture Strategist:** Finding 8.2 — "O(n) per keystroke that should be O(1)."
- **Pattern Recognition:** Finding 7.3.1 — "unnecessary allocation pattern."

**Affected location:**
- `src/widgets/tree_state.rs:101-136` — `visible_nodes()` rebuilds every call

## Proposed Solutions

### Option A: Cache flat list in TreeState (Recommended)
Store `cached_flat: Vec<FlatNode>` and `cache_valid: bool` in `TreeState`. Invalidate only on `toggle_expand()` or tree data change. Return `&[FlatNode]`.

```rust
pub fn visible_nodes(&mut self, tree: &[TreeNode]) -> &[FlatNode] {
    if !self.cache_valid {
        self.cached_flat.clear();
        self.flatten(tree, &mut self.cached_flat);
        self.cache_valid = true;
    }
    &self.cached_flat
}
```

- **Pros:** Eliminates all redundant flattening, O(1) cursor movement
- **Cons:** Must remember to invalidate on structural changes
- **Effort:** Medium
- **Risk:** Low (invalidation points are few and well-defined)

### Option B: Store only the count, search on demand
Cache `visible_count: usize` for cursor bounds. Only flatten for render and selected_target.

- **Pros:** Simpler cache, fewer invalidation concerns
- **Cons:** Still flattens twice per keypress (selected_target + render)
- **Effort:** Small
- **Risk:** Low

## Recommended Action

Option A.

## Technical Details

**Affected files:** `src/widgets/tree_state.rs`
**Components:** Tree navigation, rendering
**Invalidation triggers:** `toggle_expand()`, tree data reload in App

## Acceptance Criteria

- [ ] `visible_nodes()` only recomputes when tree structure changes
- [ ] Arrow key navigation does not allocate
- [ ] All existing tree_state tests pass
- [ ] Benchmark: <1ms per keypress at 500 sessions

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Hot-path caching is the #1 performance win |
