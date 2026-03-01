---
status: pending
priority: p2
issue_id: "019"
tags: [code-review, performance, database]
dependencies: []
---

# N+1 Query Pattern in db.rs get_tree()

## Problem Statement

`Database::get_tree()` fetches all groups, then for each group calls `sessions_for_group(gid)` as a separate SQL query. With 20 groups, that's 22 queries (1 for groups + 20 for group sessions + 1 for ungrouped).

## Findings

- **Performance Oracle:** OPT-8 (MEDIUM) — "Use a single query with LEFT JOIN."

**Affected location:**
- `src/db.rs:220-263` — `get_tree()` with per-group query loop

## Proposed Solutions

### Option A: Single JOIN query (Recommended)
```sql
SELECT g.id, g.name, g.icon, g.sort_order,
       s.session_id, s.display_name, s.cwd, ...
FROM groups g
LEFT JOIN session_groups sg ON g.id = sg.group_id
LEFT JOIN sessions s ON sg.session_id = s.session_id
ORDER BY g.sort_order, s.last_active DESC
```
Build the tree in Rust from the flat result set.

- **Pros:** 1 query instead of N+2, significantly faster at scale
- **Cons:** More complex Rust code to reconstruct tree from flat rows
- **Effort:** Medium
- **Risk:** Low

### Option B: Batch session fetch
Fetch all sessions in one query, then distribute to groups in Rust.

- **Pros:** 3 queries total (groups, all sessions, ungrouped), simpler than JOIN
- **Cons:** Still 3 queries, needs Rust-side grouping
- **Effort:** Medium
- **Risk:** Low

## Recommended Action

Option A for maximum efficiency, Option B as simpler alternative.

## Technical Details

**Affected files:** `src/db.rs`

## Acceptance Criteria

- [ ] `get_tree()` uses at most 2-3 queries regardless of group count
- [ ] Tree structure identical to current output
- [ ] All db tests pass

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Classic N+1 query pattern |
