---
status: pending
priority: p3
issue_id: "031"
tags: [code-review, duplication, architecture]
dependencies: []
---

# Duplicate Tree Traversal Helpers in app.rs

## Problem Statement

Four free functions in `app.rs` perform recursive tree traversal with near-identical match-on-variant structure:
- `find_session_cwd()` (lines 242-258)
- `find_session_in_tree()` (lines 260-276)
- `count_sessions()` (lines 278-297)
- `mark_active_sessions()` (lines 299-310)

`find_session_cwd` is a strict subset of `find_session_in_tree` — it could be implemented as `find_session_in_tree(...).and_then(|s| s.cwd...)`.

These domain operations on `TreeNode` should live closer to the type definition, not in the app coordinator.

## Findings

- **Code Simplicity:** Finding #12 — "two near-identical recursive tree-walk functions."
- **Pattern Recognition:** Finding 2.4 — "four separate recursive functions with near-identical structure."
- **Architecture Strategist:** Finding 3.2 — "domain operations that should live closer to the type definition."

## Proposed Solutions

### Option A: Consolidate and move to types.rs (Recommended)
1. Delete `find_session_cwd()`, use `find_session_in_tree()` instead
2. Move remaining tree helpers to `impl` block on a newtype or into `types.rs`
3. Consider a generic `for_each_session` iterator

- **Pros:** ~17 LOC reduction, better discoverability, cleaner app.rs
- **Cons:** Minor refactor
- **Effort:** Small
- **Risk:** Low

## Technical Details

**Affected files:** `src/app.rs`, potentially `src/types.rs`

## Acceptance Criteria

- [ ] `find_session_cwd` eliminated in favor of `find_session_in_tree`
- [ ] Tree traversal helpers are near the type definition
- [ ] All tests pass

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Domain operations belong near their types |
