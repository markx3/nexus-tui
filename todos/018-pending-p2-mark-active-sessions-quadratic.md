---
status: pending
priority: p2
issue_id: "018"
tags: [code-review, performance]
dependencies: []
---

# O(S*W) Quadratic Complexity in mark_active_sessions

## Problem Statement

`mark_active_sessions()` does a linear scan of all tmux windows for every session in the tree: `windows.iter().any(|w| w.session_id == s.session_id)`. With S sessions and W windows, this is O(S * W) string comparisons every 2 seconds.

At 500 sessions and 20 windows: 10,000 string comparisons per poll.

## Findings

- **Performance Oracle:** CRITICAL-4 (MEDIUM, becomes HIGH at scale).

**Affected location:**
- `src/app.rs:299-310` — `mark_active_sessions()` with nested linear scan

## Proposed Solutions

### Option A: HashSet pre-computation (Recommended)
Build `HashSet<&str>` from windows before the tree walk.

```rust
let active_ids: HashSet<&str> = windows.iter().map(|w| w.session_id.as_str()).collect();
```

- **Pros:** O(S + W) instead of O(S * W), trivial change
- **Cons:** None
- **Effort:** Small
- **Risk:** Low

## Technical Details

**Affected files:** `src/app.rs`

## Acceptance Criteria

- [ ] `mark_active_sessions` uses HashSet for lookup
- [ ] O(S + W) complexity

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Classic O(N*M) → O(N+M) with HashSet |
