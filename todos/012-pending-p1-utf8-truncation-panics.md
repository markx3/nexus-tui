---
status: pending
priority: p1
issue_id: "012"
tags: [code-review, correctness, security]
dependencies: []
---

# UTF-8 Truncation Panics in detail.rs and radar.rs

## Problem Statement

Two widgets perform byte-index string slicing without checking character boundaries. If a session's first message or group name contains multi-byte UTF-8 characters (emoji, CJK, accented letters) and the slice point falls mid-character, the application will **panic at runtime**, crashing the TUI.

The scanner module already has a correct `truncate()` function with `is_char_boundary()` checks, but it is not reused in the widget layer.

## Findings

- **Security Sentinel:** Findings #8 and #9 — classified as LOW severity denial-of-self.
- **Performance Oracle:** OPT-6 — flagged as correctness bug.
- **Architecture Strategist:** Finding 10.2 — "latent panic, a one-line fix."
- **Pattern Recognition:** Finding 2.2 — "the most significant bug identified in the analysis."
- **All agents agree:** This is a real bug triggerable by normal user data.

**Affected locations:**
- `src/widgets/detail.rs:90-93` — `&msg[..inner_width.saturating_sub(5)]` slices by byte index
- `src/widgets/radar.rs:161-163` — `&blip.group_name[..12]` slices by byte index

**Correct implementation already exists:**
- `src/scanner.rs:467-478` — `truncate()` with `is_char_boundary` guard

## Proposed Solutions

### Option A: Extract shared truncation utility (Recommended)
Move `truncate()` from `scanner.rs` into a new shared utility (e.g., `src/text_utils.rs` or `src/util.rs`). Use it in detail.rs, radar.rs, and scanner.rs.

- **Pros:** Single source of truth, consistent behavior, prevents future occurrences
- **Cons:** Creates a new module
- **Effort:** Small
- **Risk:** Low

### Option B: Inline char-boundary checks at each site
Add `while !s.is_char_boundary(end) { end -= 1; }` at each truncation point.

- **Pros:** No new module, minimal change
- **Cons:** Duplication persists, easy to forget in future truncation sites
- **Effort:** Small
- **Risk:** Low

## Recommended Action

Option A.

## Technical Details

**Affected files:** `src/widgets/detail.rs`, `src/widgets/radar.rs`, `src/scanner.rs`
**Components:** Widget rendering layer
**Database changes:** None

## Acceptance Criteria

- [ ] No direct byte-index slicing on user-controlled strings in any widget
- [ ] Shared `truncate()` utility exists and is used by detail.rs, radar.rs, scanner.rs
- [ ] Test with multi-byte characters (emoji, CJK) exercising the truncation boundary
- [ ] No panics when rendering sessions with non-ASCII content

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Flagged by 4 of 7 agents independently |

## Resources

- Scanner's correct implementation: `src/scanner.rs:467-478`
- Prior finding: `todos/011-complete-p3-utf8-truncation-test.md` (related test gap)
