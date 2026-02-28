---
status: pending
priority: p3
issue_id: "010"
tags: [code-review, performance]
dependencies: []
---

# BufReader Uses Default 8KB Buffer — Increase for Large JSONL Lines

## Problem Statement

At `src/scanner.rs:303`, `BufReader::new(file)` uses the default 8KB buffer. JSONL lines for assistant entries can be hundreds of KB (tool calls, code blocks). With an 8KB buffer, a 200KB line requires ~25 `read()` syscalls.

## Proposed Solutions

```rust
let reader = BufReader::with_capacity(256 * 1024, file); // 256KB
```

- **Effort:** Trivial (1 line change)
- **Risk:** None (256KB is negligible memory)

## Acceptance Criteria

- [ ] BufReader uses a larger buffer capacity

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from code review of task 02 | |
