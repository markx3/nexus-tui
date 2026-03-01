---
status: complete
priority: p1
issue_id: "032"
tags: [code-review, architecture, plan-correction]
dependencies: []
---
# Fix FocusPanel Removal Phase Sequencing

## Problem Statement
The plan removes the `FocusPanel` enum in Phase 1 (types.rs changes) but code referencing `SelectionState.focused_panel` survives until Phase 3. This creates a compile error between phases. The plan's Phase 1 success criterion is "cargo build succeeds" but removing FocusPanel while radar code still references it will fail this.

## Findings
- `FocusPanel` is defined at `src/types.rs:27-31`
- Referenced in `src/app.rs:182-186` (Tab toggle), `src/app.rs:192,201-204` (key dispatch), `src/ui.rs:63,71,78` (render focus state)
- `SelectionState.focused_panel` is used throughout `app.rs` for dispatch routing
- Tab key handler at `app.rs:181-186` and `handle_radar_key` at `app.rs:225` also depend on FocusPanel
- Found by: Architecture Strategist, Pattern Recognition Specialist

## Proposed Solutions

### Option 1: Defer FocusPanel removal to Phase 2 (Recommended)
**Approach:** Move FocusPanel enum removal from Phase 1 to Phase 2, where radar.rs, radar_state.rs, activity.rs, and all radar references are deleted in the same commit.
**Pros:** Clean single-commit removal, no intermediate compile errors, phases remain shippable checkpoints
**Cons:** FocusPanel lives slightly longer than ideal
**Effort:** Plan text change only (0 code effort)
**Risk:** Low

### Option 2: Remove all FocusPanel usages in Phase 1
**Approach:** Pull forward the removal of Tab handler, handle_radar_key, SelectionState.focused_panel, and all focus-based dispatch into Phase 1 alongside the enum deletion.
**Pros:** Removes dead concept early
**Cons:** Phase 1 becomes larger and touches app.rs dispatch logic before the new Alt-based routing is ready, creating a broken intermediate state
**Effort:** 1-2 hours additional Phase 1 work
**Risk:** Medium — temporarily breaks key dispatch

## Technical Details
**Affected files:** src/types.rs, src/app.rs, src/ui.rs
**Related components:** Key dispatch routing, render focus state, Tab handler

## Acceptance Criteria
- [ ] FocusPanel removal and all dependent code changes happen in the same phase
- [ ] `cargo build` succeeds at the end of every phase
- [ ] No dead code references to FocusPanel remain after the removal phase

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Architecture Strategist + Pattern Recognition Specialist
**Actions:** Identified phase sequencing conflict between enum removal and code that references it
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** Moved FocusPanel removal from Phase 1 to Phase 2 alongside radar/activity deletion. Added comprehensive removal checklist including handle_radar_key, Tab handler, advance_sweep, compute_blips, select_by_session_id, and ui.rs filter logic.
