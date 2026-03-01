---
status: pending
priority: p2
issue_id: "020"
tags: [code-review, performance, database]
dependencies: []
---

# N+1 Query Pattern in grouping::apply_rules

## Problem Statement

For each unassigned session, `apply_rules` makes individual DB calls:
1. `db.get_session_cwd(session_id)` — one SELECT per session
2. `db.get_group_id_by_name(&rule.group)` — one SELECT per rule match
3. `db.assign_session_to_group(...)` — one INSERT per match

With 200 unassigned sessions and 5 rules: ~200 individual SELECTs + up to 200 INSERTs.

## Findings

- **Performance Oracle:** OPT-7 (MEDIUM) — "Batch the cwd lookups into a single query."

**Affected location:**
- `src/grouping.rs:13-43` — `apply_rules()` with per-session DB calls

## Proposed Solutions

### Option A: Batch operations (Recommended)
1. Fetch all `(session_id, cwd)` pairs in one query
2. Pre-fetch all group IDs by name once
3. Wrap all assignments in a single transaction

- **Pros:** Reduces ~400 queries to ~3, dramatically faster at scale
- **Cons:** Changes function signature (needs list of cwds, not per-session lookup)
- **Effort:** Medium
- **Risk:** Low

## Technical Details

**Affected files:** `src/grouping.rs`, `src/db.rs` (may need batch query methods)

## Acceptance Criteria

- [ ] `apply_rules` uses batched DB operations
- [ ] All assignments wrapped in a single transaction
- [ ] All grouping tests pass
- [ ] Performance: <50ms for 500 sessions with 10 rules

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Batch DB operations at module boundaries |
