---
status: pending
priority: p2
issue_id: "017"
tags: [code-review, simplicity, yagni]
dependencies: []
---

# Remove Unimplemented Tree Input Modes (YAGNI)

## Problem Statement

`TreeInputMode` has 6 variants (Normal, Search, CreateGroup, Rename, MoveMode, ConfirmDelete) but only `Normal` produces any visible effect. The `TreeAction` enum has 9 variants, but `app.rs:handle_tree_action()` only handles `Select`, `ToggleExpand`, `ScrollDown`, `ScrollUp` — all others fall through to `_ => {}`.

A user pressing `d` enters ConfirmDelete mode but sees no prompt. The app swallows all keys until Esc. Same for `/` (search), `n` (create), `r` (rename), `m` (move).

## Findings

- **Code Simplicity:** Finding #3 (HIGH) — "YAGNI: plumbing exists for features that do not function."
- **Agent-Native Reviewer:** Finding 5 — "Multiple tree actions silently dropped."

**~45 LOC of dead feature scaffolding.**

## Proposed Solutions

### Option A: Strip to Normal-only (Recommended)
Remove all modes except `Normal`. Remove corresponding `TreeAction` variants. Remove `handle_search_key()`, `handle_confirm_delete_key()`, `handle_modal_key()`, and `search_query` field. Keep only `Normal` mode with `Select`, `ToggleExpand`, `ScrollUp`, `ScrollDown`.

- **Pros:** ~45 LOC reduction, eliminates user-confusing dead keys, cleaner state machine
- **Cons:** Must re-add when features are implemented
- **Effort:** Small
- **Risk:** Low

### Option B: Implement the missing handlers
Wire up Search, Create, Rename, Move, Delete in `app.rs:handle_tree_action()`.

- **Pros:** Features actually work
- **Cons:** Significant effort, needs UI for each mode (prompts, input fields), scope creep
- **Effort:** Large
- **Risk:** Medium

## Recommended Action

Option A — remove the dead scaffolding now. Add it back when implementing each feature properly.

## Technical Details

**Affected files:** `src/widgets/tree_state.rs`, `src/app.rs`

## Acceptance Criteria

- [ ] `TreeInputMode` has only `Normal` variant (or enum removed entirely)
- [ ] No keys silently trap input without visual feedback
- [ ] Keys d, /, n, r, m are either unbound or produce no mode change
- [ ] All existing tests pass

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Dead feature scaffolding confuses users and agents |
