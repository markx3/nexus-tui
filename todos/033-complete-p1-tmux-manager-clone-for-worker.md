---
status: complete
priority: p1
issue_id: "033"
tags: [code-review, architecture, threading]
dependencies: []
---
# TmuxManager Needs Clone for Capture Worker Thread

## Problem Statement
The capture worker pseudocode takes `tmux: TmuxManager` by value, but the main thread also needs a TmuxManager for `send_keys()`, `resize_pane()`, `kill_session()`, etc. The plan does not address TmuxManager ownership between threads.

## Findings
- `TmuxManager` at `src/tmux.rs:12-14` is a simple struct holding only `socket_name: String`
- `App` owns the only instance at `src/app.rs:31`
- Worker thread needs its own instance for `capture_pane()` calls
- TmuxManager does not derive Clone
- Each tmux subprocess invocation is independent (no shared state between calls)
- Found by: Architecture Strategist, Performance Oracle, Pattern Recognition Specialist (all three flagged this)

## Proposed Solutions

### Option 1: Derive Clone on TmuxManager (Recommended)
**Approach:** Add `#[derive(Clone)]` to TmuxManager. Clone it when spawning the capture worker thread.
**Pros:** Trivial one-line change, each thread gets its own instance, no shared state concerns
**Cons:** None — TmuxManager only holds a String
**Effort:** 5 minutes
**Risk:** Low

### Option 2: Use Arc<TmuxManager>
**Approach:** Wrap in Arc, share between threads.
**Pros:** Single instance
**Cons:** Unnecessary overhead — TmuxManager is cheap to clone, Arc adds indirection
**Effort:** 15 minutes
**Risk:** Low

## Technical Details
**Affected files:** src/tmux.rs (add derive), src/app.rs (clone for worker)

## Acceptance Criteria
- [ ] TmuxManager derives Clone
- [ ] Capture worker receives its own TmuxManager instance
- [ ] Main thread retains its own instance for send_keys/resize_pane/etc.

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Architecture Strategist, Performance Oracle, Pattern Recognition Specialist
**Actions:** All three agents independently identified this ownership gap
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** Added derive(Clone) task to Phase 1. Updated worker pseudocode to show tmux_manager.clone(). InteractorState also holds cloned TmuxManager for send_keys/resize/paste.
