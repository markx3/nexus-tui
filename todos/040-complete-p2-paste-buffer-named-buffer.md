---
status: complete
priority: p2
issue_id: "040"
tags: [code-review, security]
dependencies: []
---
# Use Named tmux Buffer for Paste Operations

## Problem Statement
The plan uses `tmux load-buffer -` + `tmux paste-buffer -t <session>` as two separate invocations with the default tmux buffer as shared state. Between the two calls, another process could overwrite the default buffer (TOCTOU race). Also, `load_buffer_and_paste` should be a separate TmuxManager method (not bundled into send_keys) per the existing per-operation method pattern.

## Findings
- Default tmux buffer is shared state — any tmux client can overwrite it between load and paste
- Nexus uses isolated socket `-L nexus`, but multiple nexus instances or tmux commands targeting the same socket could race
- The `-d` flag on paste-buffer deletes the named buffer after use (cleanup)
- Existing TmuxManager has one method per tmux operation (launch, resume, list, kill) — paste should follow this pattern
- Found by: Security Sentinel (MEDIUM #5), Code Simplicity Reviewer, Pattern Recognition Specialist

## Proposed Solutions

### Option 1: Named buffer with cleanup (Recommended)
**Approach:** Use `tmux load-buffer -b nexus-paste -` then `tmux paste-buffer -b nexus-paste -t <session> -d`. Keep as a separate `load_buffer_and_paste()` method on TmuxManager.
**Pros:** No TOCTOU race, auto-cleanup with -d flag, follows existing method pattern
**Cons:** None meaningful
**Effort:** 30 minutes
**Risk:** Low

## Technical Details
**Affected files:** src/tmux.rs
**Note:** Also validate paste content size (1MB limit) at the TmuxManager boundary

## Acceptance Criteria
- [ ] Paste uses named buffer `nexus-paste` (not default buffer)
- [ ] `-d` flag deletes buffer after paste
- [ ] `load_buffer_and_paste` is a separate method on TmuxManager
- [ ] Paste content size validated at 1MB limit

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Security Sentinel, Pattern Recognition Specialist
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** Updated paste handling to use named buffer nexus-paste with -b and -d flags. Made load_buffer_and_paste a separate TmuxManager method (4th method in Phase 1).
