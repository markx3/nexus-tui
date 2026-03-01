---
status: complete
priority: p3
issue_id: "042"
tags: [code-review, plan-correction]
dependencies: []
---
# Miscellaneous Plan Corrections

## Problem Statement
Multiple minor issues identified across the plan that should be corrected before implementation begins.

## Findings

1. **State struct naming inconsistency** — `InteractorState` (shortened) vs file `session_interactor_state.rs` vs panel type `SessionInteractor`. Existing pattern: `TreeState` in `tree_state.rs`, `RadarState` in `radar_state.rs`. Should be `SessionInteractorState` or rename files to `interactor.rs`/`interactor_state.rs`.

2. **Missing Role enum definition** — Plan mentions `ConversationTurn { role: Role, content: String }` but does not list `pub enum Role { Human, Assistant }` as an item to add to types.rs.

3. **CaptureResult placement unspecified** — Used by both worker and InteractorState but never says where it's defined. Should be in session_interactor_state.rs (internal impl detail).

4. **chrono::Local undeclared dependency** — Plan says to use `chrono::Local` for top bar date but does not add chrono to Cargo.toml. Existing codebase uses src/time_utils.rs without chrono.

5. **Missing derive annotations** — `SendKeysArgs` and `TmuxKeyName` shown without derives. Need at minimum `Debug, Clone` for SendKeysArgs and `Debug, Clone, Copy, PartialEq, Eq` for TmuxKeyName.

6. **Missing removal checklist items** — Plan does not mention removing: `handle_radar_key` method, Tab key handler, `advance_sweep()` call, `radar_state.compute_blips()` calls, `radar_state.select_by_session_id()` calls, filtering logic in ui.rs lines 81-88.

7. **scanner.rs reference may be stale** — Plan references `src/scanner.rs` in internal references but this file may not exist in current source tree (was possibly renamed/refactored). Verify before implementation.

8. **TmuxKeyName::F(u8) and Ctrl(char) need validation** — F(0), F(13+) are invalid; Ctrl with non-alpha is invalid. Either validate at construction or document that only key_event_to_send_args creates these values.

9. **Conversation log JSONL path resolution undefined** — Plan says to parse JSONL but doesn't specify how conversation.rs locates the JSONL file given a SessionSummary. SessionSummary has no jsonl_path field.

10. **render_detail focused parameter** — Plan removes FocusPanel but doesn't specify what focused value render_detail receives. Should always be false (or remove the parameter).

## Proposed Solutions

### Option 1: Fix all in a single plan revision pass (Recommended)
**Approach:** Update the plan document to address all 10 items before implementation begins.
**Effort:** 30 minutes
**Risk:** Low

## Acceptance Criteria
- [ ] All 10 items addressed in revised plan
- [ ] Naming is consistent across files, structs, and enums
- [ ] All dependencies listed in Cargo.toml section
- [ ] Removal checklist is complete

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Pattern Recognition Specialist, Architecture Strategist
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** All 10 items addressed: (1) Files renamed to interactor.rs/interactor_state.rs. (2) Role enum added to types.rs changes. (3) CaptureResult placed in interactor_state.rs. (4) chrono replaced with time_utils.rs. (5) Derives added to SendKeysArgs. (6) Removal checklist expanded. (7) scanner.rs reference updated to db.rs. (8) F(u8)/Ctrl(char) validation documented via match arms. (9) jsonl_path added to SessionSummary. (10) focused: false documented for detail panel.
