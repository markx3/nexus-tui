---
status: pending
priority: p2
issue_id: "026"
tags: [code-review, performance]
dependencies: ["016"]
---

# Per-Frame String Allocations + SystemTime::now() in Tree Rendering

## Problem Statement

For every visible tree row, every frame at 60 FPS:
- `"  ".repeat(depth)` — new String allocation
- `format!(" ({})", count)` — new String
- `format!(" {}", name)` — new String
- `relative_time()` — new String + `SystemTime::now()` syscall
- `format!("  {}", rel_time)` — new String

At 20 visible rows: ~720 String allocations/sec + ~1200 `SystemTime::now()` syscalls/sec.

Additionally, `current_date_string()` in top_bar.rs recomputes the Hinnant algorithm every frame for a value that changes once per day.

## Findings

- **Performance Oracle:** OPT-1 (MEDIUM) and OPT-2 (MEDIUM) — "pre-compute indent strings, cache relative times."

**Depends on:** #016 (consolidate datetime) — relative_time caching is easier after consolidation.

## Proposed Solutions

### Option A: Cache relative times + pre-compute indents (Recommended)
1. Capture `now_epoch` once per frame, pass to widgets
2. Cache relative time strings — only recompute when data changes (not every frame)
3. Pre-compute indent strings for depths 0-4 as `const` slices
4. Cache date string in top_bar, recompute once per minute

- **Pros:** Eliminates majority of per-frame allocations and syscalls
- **Cons:** Adds caching complexity
- **Effort:** Medium
- **Risk:** Low

## Technical Details

**Affected files:** `src/widgets/tree.rs`, `src/widgets/top_bar.rs`, `src/ui.rs`

## Acceptance Criteria

- [ ] `SystemTime::now()` called at most once per frame (not per-row)
- [ ] Indent strings are static, not allocated per frame
- [ ] Relative time strings cached and only recomputed on data change
- [ ] Date string recomputed at most once per minute

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Frame-budget awareness at 60 FPS |
