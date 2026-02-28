---
status: pending
priority: p2
issue_id: "007"
tags: [code-review, architecture]
dependencies: []
---

# Quick Scan Produces Incomplete SessionInfo With No Provenance Marker

## Problem Statement

`scan_quick()` returns `SessionInfo` structs where `message_count` may be partial, `token_usage` may be zero, and `last_active` may fall back to file mtime. Downstream consumers have no way to distinguish "this session truly has 0 output tokens" from "we only read 50 lines."

## Findings

- **Architecture Strategist:** Flagged as P2 — suggests either a `scan_mode` field on SessionInfo or wrapping unreliable fields in `Option`.

## Proposed Solutions

### Option A: Add a `is_complete: bool` field to SessionInfo
```rust
pub struct SessionInfo {
    // ...existing fields...
    pub is_complete: bool,  // true for full scan, false for quick scan
}
```
- **Pros:** Minimal API change, consumers can check provenance
- **Cons:** Single bool is coarse-grained
- **Effort:** Trivial
- **Risk:** None

### Option B: Wrap quick-scan-unreliable fields in Option
Change `message_count: u32` to `message_count: Option<u32>` (None for quick scan).
- **Pros:** Type system enforces awareness
- **Cons:** More disruptive API change, makes downstream code more verbose
- **Effort:** Small
- **Risk:** Low

### Option C: Defer until integration (document behavior for now)
- **Pros:** No change until real consumers exist
- **Cons:** Technical debt
- **Effort:** Trivial
- **Risk:** Low

## Acceptance Criteria

- [ ] Consumers can distinguish quick-scan partial data from full-scan complete data
- [ ] Or: behavior is documented in doc comments on `scan_quick()`

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from code review of task 02 | |
