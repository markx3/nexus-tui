---
status: pending
priority: p2
issue_id: "004"
tags: [code-review, architecture]
dependencies: []
---

# scan() Returns Unsorted Vec — Downstream Consumers Will All Need to Sort

## Problem Statement

The returned `Vec<SessionInfo>` has no defined ordering. All three downstream consumers (session tree, radar, info panels) will almost certainly need sessions sorted by `last_active` descending. Without a sorting contract, each consumer must implement its own sort.

## Findings

- **Architecture Strategist:** Flagged as P1 — "Add a sort by `last_active` descending, or document the contract explicitly."

**Affected location:**
- `src/scanner.rs:294` — end of `scan()` function, before `Ok(sessions)`

## Proposed Solutions

### Option A: Sort by `last_active` descending before returning (Recommended)
```rust
sessions.sort_unstable_by(|a, b| b.last_active.cmp(&a.last_active));
Ok(sessions)
```
- **Pros:** Single line, all consumers get sorted data for free, ISO 8601 string comparison works for UTC timestamps
- **Cons:** Imposes a contract. If a consumer wants a different sort, it re-sorts (cheap).
- **Effort:** Trivial
- **Risk:** None

### Option B: Document that ordering is unspecified
- **Pros:** No code change, maximum flexibility
- **Cons:** Every downstream consumer must sort independently
- **Effort:** Trivial
- **Risk:** Low

## Acceptance Criteria

- [ ] `scan()` returns sessions sorted by `last_active` descending
- [ ] Or: ordering contract is documented in doc comments

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from code review of task 02 | |
