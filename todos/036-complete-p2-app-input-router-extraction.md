---
status: complete
priority: p2
issue_id: "036"
tags: [code-review, architecture]
dependencies: []
---
# Extract Input Routing from App into InteractorState

## Problem Statement
The App struct already has 22 fields and handles tree CRUD, text input, confirm dialogs, group picker, tmux reconciliation, and tree refresh. The plan adds tmux input forwarding, paste handling, resize debounce, and capture polling directly to App::handle_events(). This will push app.rs past 1,200 lines with deeply interleaved concerns.

## Findings
- App struct at src/app.rs has 22 fields (lines 22-54)
- handle_normal_key currently dispatches based on FocusPanel — straightforward
- After changes: must distinguish Alt vs non-Alt keys, forward to tmux, handle paste, handle resize, poll captures
- Architecture Strategist recommends InteractorState own the full input-forwarding pipeline
- App should call `interactor_state.route_input(event)` and receive back "handled" or "nexus command"

## Proposed Solutions

### Option 1: Expand InteractorState as input router (Recommended)
**Approach:** InteractorState owns: key event classification (Alt vs forward), keystroke dispatch to tmux, paste validation + load-buffer, resize debounce, capture polling. App::handle_events() calls interactor_state.handle_event() and receives a `RouteResult` enum: `Forwarded`, `NexusCommand(cmd)`, `Ignored`.
**Pros:** Keeps App as orchestrator, interactor concerns stay encapsulated, testable in isolation
**Cons:** InteractorState needs reference to TmuxManager (or owns a clone)
**Effort:** 3-4 hours
**Risk:** Low

### Option 2: Keep everything in App
**Approach:** Add all forwarding logic directly to App::handle_events() as the plan currently specifies.
**Pros:** Simpler initial implementation, no new abstractions
**Cons:** App grows to 1200+ lines, interleaved concerns, harder to test
**Effort:** 2 hours (but higher maintenance cost)
**Risk:** Medium — maintainability debt

## Technical Details
**Affected files:** src/app.rs, src/widgets/session_interactor_state.rs
**Related:** App already handles too many concerns — this is a pre-existing issue amplified by the new feature

## Acceptance Criteria
- [ ] App::handle_events() delegates to InteractorState for all tmux-related input handling
- [ ] App does not directly call tmux.send_keys() or tmux.load_buffer_and_paste()
- [ ] InteractorState is testable without a full App instance

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Architecture Strategist
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** Rewrote Phase 3 event loop to delegation pattern. InteractorState.route_event() returns RouteResult enum. App never directly calls tmux.send_keys() or load_buffer_and_paste(). InteractorState owns all tmux input handling with cloned TmuxManager.
