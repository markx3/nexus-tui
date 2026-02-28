---
status: pending
priority: p3
issue_id: "009"
tags: [code-review, quality]
dependencies: []
---

# Remove Manual fs::remove_dir_all Cleanup From Tests

## Problem Statement

Every test ends with `let _ = fs::remove_dir_all(&projects);` (12 occurrences). This is redundant because `create_fixture_dir` already calls `remove_dir_all` at the start, and if a test panics the cleanup line never runs anyway.

## Proposed Solutions

Remove all 12 `let _ = fs::remove_dir_all(&projects);` lines.

- **Effort:** Trivial
- **Risk:** None (cleanup-at-start already handles it)

## Acceptance Criteria

- [ ] No manual cleanup lines remain in tests
- [ ] All tests still pass

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from code review of task 02 | |
