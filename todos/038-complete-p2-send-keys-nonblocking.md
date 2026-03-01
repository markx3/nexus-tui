---
status: complete
priority: p2
issue_id: "038"
tags: [code-review, performance]
dependencies: []
---
# Make send-keys Non-Blocking on Main Thread

## Problem Statement
The plan keeps `tmux send-keys` synchronous on the main thread (2-5ms per call). The 16ms tick rate means a single send-keys call consumes 12-31% of the frame budget. Occasional 10-20ms stalls from fork() on a loaded system will cause visible input lag. Users comparing the interactor to native tmux will notice.

## Findings
- `Command::new("tmux").status()` involves fork() + exec() + wait — 2-5ms typical, 10-20ms spikes
- Current TICK_RATE = 16ms (src/app.rs line 19)
- During fast typing (10 chars/sec), send-keys burns 20-50ms/sec of main thread time
- Variance is the problem: occasional stalls create perceptible keystroke-to-display lag
- Found by: Performance Oracle (Critical #2.3)

## Proposed Solutions

### Option 1: Fire-and-forget with Command::spawn() (Recommended)
**Approach:** Use `Command::spawn()` instead of `Command::status()` for send-keys. Don't wait for exit status.
**Pros:** Zero blocking on main thread, simplest implementation
**Cons:** Loses exit status (can't detect immediate failures), orphaned processes if tmux server dies
**Effort:** 30 minutes
**Risk:** Low — validate session name before calling, rely on capture worker for death detection

### Option 2: Dedicated sender thread with mpsc channel
**Approach:** Mirror capture worker pattern for send-keys. Main thread sends keystroke events via channel, sender thread dispatches to tmux.
**Pros:** Full error handling, ordered delivery guaranteed
**Cons:** More complex, another thread to manage
**Effort:** 2 hours
**Risk:** Low

## Technical Details
**Affected files:** src/tmux.rs (send_keys method)

## Acceptance Criteria
- [ ] send-keys calls do not block the main event loop
- [ ] Keystroke input latency is not perceptible compared to native tmux

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Performance Oracle
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** Updated send_keys method to use Command::spawn() (fire-and-forget). Updated worker description and Phase 1 method specification.
