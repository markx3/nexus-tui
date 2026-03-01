---
status: complete
priority: p2
issue_id: "041"
tags: [code-review, performance]
dependencies: []
---
# Use Adaptive Capture Polling and Fix Worker Busy-Loop

## Problem Statement
Two related issues: (1) The capture worker uses fixed 80ms polling — wasteful on idle sessions, too slow during active output. (2) When no session is selected, the worker busy-loops with 50ms sleep.

## Findings
- Fixed 80ms sleep wastes CPU on idle sessions (content unchanged for minutes) and is slower than needed during active Claude Code streaming
- When current_session is empty, worker spins at 20Hz doing try_recv + sleep — pure waste
- Performance Oracle recommends exponential backoff: 30ms during activity, up to 500ms when idle
- Worker should use `recv()` (blocking) when no session is selected
- Also: event drain per frame is needed — current loop reads one event per poll() call, preventing keystroke batching from working
- Found by: Performance Oracle

## Proposed Solutions

### Option 1: Adaptive backoff + blocking recv for empty session (Recommended)
**Approach:** When session is empty, use `session_rx.recv()` (blocking wait). When capturing, use exponential backoff: start at 30ms after content change, double up to 500ms when unchanged.
**Pros:** Near-zero CPU when idle, 33fps during active output, no busy-loop waste
**Cons:** Slightly more complex worker logic
**Effort:** 1 hour
**Risk:** Low

## Technical Details
**Affected files:** capture worker function
**Also needed:** Change event loop to drain all pending events per frame (while event::poll(Duration::ZERO)), not just one. Without this, keystroke batching never triggers.

## Acceptance Criteria
- [ ] Worker uses recv() when no session is selected (no CPU burn)
- [ ] Worker adapts polling interval based on content change frequency
- [ ] Event loop drains all pending events per frame

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Performance Oracle
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** Replaced fixed 80ms with adaptive backoff (30ms-500ms) in worker pseudocode. Replaced empty-session busy-loop with blocking recv(). Added event drain to Phase 3 event loop.
