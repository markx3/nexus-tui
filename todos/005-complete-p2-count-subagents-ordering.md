---
status: pending
priority: p2
issue_id: "005"
tags: [code-review, performance]
dependencies: []
---

# count_subagents() Called Before parse_session_file() — Wasted I/O for Skipped Sessions

## Problem Statement

At `src/scanner.rs:270`, `count_subagents()` is called for every JSONL file before `parse_session_file()`. Sessions that are later filtered out (snapshot-only, empty files) still pay the cost of a `stat()` syscall on the subagents directory. Moving the call after parsing succeeds avoids wasted I/O.

## Findings

- **Performance Oracle:** Flagged as O4 — "defers the directory scan until after confirming the session is valid."
- **Architecture Strategist:** Confirmed — "computing derived data before knowing if the session is valid is inverted."

## Proposed Solutions

### Option A: Move count_subagents after parse succeeds (Recommended)
```rust
match parse_session_file(&file_path, &session_id, mode) {
    Ok(Some(builder)) => {
        let subagent_count = count_subagents(&project_path, &session_id);
        sessions.push(builder.build(
            project_dir.clone(), subagent_count, fallback_ts, file_path,
        ));
    }
    // ...
}
```
- **Pros:** Eliminates unnecessary `stat` calls for skipped sessions, free correctness fix
- **Cons:** None
- **Effort:** Trivial (move 1 line)
- **Risk:** None

## Acceptance Criteria

- [ ] `count_subagents()` is called only for sessions that pass `has_meaningful_data()`
- [ ] `fallback_ts` computation similarly deferred (or left as-is since it's cheap)

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from code review of task 02 | |
