---
status: pending
priority: p2
issue_id: "022"
tags: [code-review, correctness, database]
dependencies: []
---

# is_complete/is_active Semantic Confusion in Database

## Problem Statement

In `db.rs:133`, `is_complete` (which indicates whether a full scan was performed) is inverted and stored as the `is_active` DB column:

```rust
!s.is_complete as i32, // active = not yet completed
```

This conflates "scan completeness" with "session is active (has tmux window)". Meanwhile, `app.rs:mark_active_sessions()` sets `is_active` on the in-memory `SessionSummary` based on actual tmux window presence. The DB value is misleading and semantically wrong.

## Findings

- **Code Simplicity:** Finding #11 (MEDIUM) — "The DB value is misleading and likely wrong."
- **Architecture Strategist:** Finding 7.4 (LOW) — "overloaded meaning, confusing."

**Affected locations:**
- `src/db.rs:133` — stores scan mode as "active"
- `src/app.rs:299-310` — overrides with tmux state at runtime

## Proposed Solutions

### Option A: Default is_active to 0, let runtime set it (Recommended)
Store `is_active = 0` on all inserts. The runtime `mark_active_sessions()` is the sole source of truth for activity. Remove the `is_complete` → `is_active` inversion.

- **Pros:** Clean semantics, single source of truth
- **Cons:** Minor schema behavior change
- **Effort:** Small
- **Risk:** Low

### Option B: Remove is_active from DB entirely
Since activity is transient tmux state, don't persist it.

- **Pros:** Cleanest semantics
- **Cons:** Requires schema migration (drop column or ignore it)
- **Effort:** Medium
- **Risk:** Low

## Recommended Action

Option A — simplest fix.

## Technical Details

**Affected files:** `src/db.rs`

## Acceptance Criteria

- [ ] DB `is_active` column reflects intended semantics or is removed
- [ ] No `is_complete` → `is_active` inversion in upsert

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Semantic column misuse creates confusion |
