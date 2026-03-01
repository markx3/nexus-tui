---
status: pending
priority: p2
issue_id: "016"
tags: [code-review, architecture, duplication]
dependencies: []
---

# Consolidate Triplicated Date/Time Implementations

## Problem Statement

Three modules independently implement epoch-to-date and date-to-epoch conversions using different algorithms, type signatures, and naming conventions:

1. `src/scanner.rs:403-465` — `file_mtime_iso()`, `days_to_ymd()`, `is_leap()` (loop-based, u64 types)
2. `src/widgets/tree.rs:27-78` — `parse_seconds_ago()`, `simple_utc_to_epoch()`, `is_leap_year()` (loop-based, i64 types)
3. `src/widgets/top_bar.rs:62-85` — `current_date_string()` (Howard Hinnant O(1) algorithm)

This also causes a cross-widget coupling: `radar_state.rs` imports `relative_time` from `tree.rs`.

## Findings

- **Architecture Strategist:** Finding 3.1 (MEDIUM) — "consolidate into a single module."
- **Pattern Recognition:** Finding 2.1 (MEDIUM) — "three separate implementations, each with slightly different algorithms."
- **Code Simplicity:** Finding #1 (HIGH) — "~80 LOC reduction."
- **Performance Oracle:** OPT-3 — "O(Y) loop from 1970 vs O(1) Hinnant algorithm."

**~100 LOC of duplication across three files.**

## Proposed Solutions

### Option A: New `src/time_utils.rs` module (Recommended)
Create a single module with the Hinnant algorithm (O(1), most compact) providing:
- `epoch_to_ymd(secs: u64) -> (i64, u64, u64)`
- `ymd_to_epoch(year, month, day, hour, min, sec) -> u64`
- `epoch_to_iso(secs: u64) -> String`
- `relative_time(iso_ts: &str) -> String`
- `is_stale(iso_ts: &str, threshold_secs: u64) -> bool`

Remove date logic from scanner.rs, tree.rs, and top_bar.rs. Fix `radar_state.rs` import to use new module.

- **Pros:** Single source of truth, eliminates cross-widget coupling, ~60 net LOC reduction
- **Cons:** New module to maintain
- **Effort:** Medium
- **Risk:** Low

### Option B: Use `EpochSeconds(u64)` newtype for `last_active`
In addition to Option A, change `SessionSummary.last_active` from `String` to `EpochSeconds(u64)`. Parse once at scan time, format for display only in widgets.

- **Pros:** Eliminates all per-frame string parsing, type-safe comparison/sorting
- **Cons:** Larger refactor touching types.rs, db.rs, scanner.rs, widgets
- **Effort:** Large
- **Risk:** Medium

## Recommended Action

Option A now, Option B as follow-up.

## Technical Details

**Affected files:** `src/scanner.rs`, `src/widgets/tree.rs`, `src/widgets/top_bar.rs`, `src/widgets/radar_state.rs`, new `src/time_utils.rs`

## Acceptance Criteria

- [ ] Single date/time utility module exists
- [ ] No date conversion logic in scanner.rs, tree.rs, or top_bar.rs
- [ ] `radar_state.rs` no longer imports from `widgets::tree`
- [ ] All existing tests pass
- [ ] Hinnant O(1) algorithm used (no year loops)

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Flagged by 4 of 7 agents independently |
