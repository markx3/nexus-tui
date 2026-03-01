---
status: complete
priority: p3
issue_id: "043"
tags: [code-review, simplification]
dependencies: []
---
# Simplify Plan: Remove YAGNI Items

## Problem Statement
The plan contains several premature optimizations and defensive structures that should be deferred until real usage proves they are needed. Estimated ~200-300 LOC reduction from removing these items.

## Findings

1. **TmuxKeyName exhaustive enum is over-engineered** — The security argument is already handled by SendKeysArgs::Literal using -l flag. For Named keys, a `&'static str` from match arms on KeyCode is equally injection-proof (compile-time constants from match). Use `SendKeysArg::Named(&'static str)` instead of a full enum.

2. **Conversation log LRU cache + lazy-loading** — Parsing a JSONL file for 50-100 text turns is sub-millisecond. Store parsed result for current session only. Re-parse on switch. No LRU, no lazy loading.

3. **SessionContent::Empty variant unnecessary** — An empty String or empty Vec already communicates "no content". Two variants (Live, Log) are enough. Check is_empty() in render.

4. **CaptureResult::SessionDead with error classification** — Existing 2-second tmux poll already detects dead sessions via reconcile_tmux_state. Worker can send Option<String> — None means no new content. No special death detection needed.

5. **Keystroke batching (per-frame)** — Crossterm delivers events one at a time. Multiple keystrokes per 16ms frame is rare during normal typing. Send individually. Profile later.

6. **"Immediate capture after send-keys"** — Extra signal channel for ~40ms latency reduction. The 80ms poll cycle is fast enough. Remove this optimization.

7. **fxhash dependency for content hashing** — Simple byte equality (last_raw == current_raw) achieves the same result for <10KB payloads. Add fxhash only if profiling shows string comparison is slow.

8. **Image paste implementation details in deferred section** — Plan spends 7 lines detailing $XDG_RUNTIME_DIR, mode 0700, uuid filenames for a deferred feature. One sentence is sufficient.

9. **Group auto-selection conflicts with group CRUD** — Auto-advancing cursor when landing on a group means user can never select a group node for rename/delete operations. Show empty interactor with "Select a session" message instead.

## Proposed Solutions

### Option 1: Revise plan to remove items 1-8, reconsider item 9 (Recommended)
**Approach:** Strip the 8 YAGNI items from the plan. For group auto-selection (#9), discuss with user — it conflicts with existing group rename/delete flows.
**Effort:** 30 minutes (plan text changes)
**Risk:** Low — each item can be re-added later if needed

## Acceptance Criteria
- [ ] Plan stripped of premature optimizations
- [ ] Group node selection behavior decided and documented
- [ ] Each removed item documented as "deferred until proven needed"

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Code Simplicity Reviewer
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** All 9 items applied: (1) TmuxKeyName replaced with Named(&'static str). (2) LRU cache removed, current session only. (3) SessionContent::Empty removed. (4) CaptureResult::SessionDead simplified to Option. (5) Keystroke batching removed. (6) Immediate capture after send-keys removed. (7) fxhash replaced with byte equality. (8) Image paste deferred section trimmed to one sentence. (9) Group auto-selection removed, show empty state instead.
