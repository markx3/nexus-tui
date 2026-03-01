---
status: complete
priority: p1
issue_id: "035"
tags: [code-review, security, plan-correction]
dependencies: []
---
# Consolidate Session Name Validation and Fix Regex

## Problem Statement
Three issues intersect: (1) `sanitize_tmux_name()` is duplicated in app.rs and main.rs, (2) the plan's validation regex `[a-zA-Z0-9_.-]` allows `.` which is a tmux target separator (regression from existing sanitizer), (3) existing code passes unsanitized names to tmux `-t` in some paths (kill_session CLI, resume_session from DB). Additionally, per existing todo #013, the current sanitizer truncates to 8 chars causing identity collisions.

## Findings
- `sanitize_tmux_name` exists at `src/app.rs:804-808` AND `src/main.rs:185-189` — identical copies
- Existing sanitizer allows `[a-zA-Z0-9-]` only (no `.` or `_`) — this is safer
- Plan's regex `[a-zA-Z0-9_.-]` adds `.` which is a tmux session:window.pane separator
- `kill_session` CLI path at `main.rs:100` passes user args directly to tmux with no sanitization
- `resume_session` at `app.rs:645` uses tmux_name from DB without re-validating
- Todo #013 documents that 8-char truncation causes session identity collisions
- Validation belongs at the TmuxManager API boundary (defense-in-depth), not at call sites
- Found by: Security Sentinel (HIGH #1, MEDIUM #3), Pattern Recognition, Learnings Researcher

## Proposed Solutions

### Option 1: Consolidate in tmux.rs with boundary validation (Recommended)
**Approach:** Move sanitize_tmux_name to tmux.rs. Add a `validate_target()` method that rejects invalid names at the TmuxManager API boundary. Use `[a-zA-Z0-9_-]` (allow underscore, exclude period). Every TmuxManager method validates its session name parameter before use.
**Pros:** Defense-in-depth, single source of truth, prevents injection in all code paths
**Cons:** Slightly more work upfront
**Effort:** 1-2 hours
**Risk:** Low

### Option 2: Validate only at call sites
**Approach:** Keep sanitizer where it is, add validation where missing.
**Pros:** Less refactoring
**Cons:** Easy to miss a call site, no defense-in-depth
**Effort:** 30 minutes
**Risk:** Medium — future code paths may forget validation

## Technical Details
**Affected files:** src/tmux.rs, src/app.rs, src/main.rs
**Related:** Todo #013 (tmux session name mismatch — truncation to 8 chars)
**Regex:** Use `[a-zA-Z0-9_-]` — exclude `.` (tmux separator) but allow `_` (common in names)

## Acceptance Criteria
- [ ] Single `sanitize_tmux_name` function in tmux.rs
- [ ] `validate_target()` called at TmuxManager API boundary for all methods
- [ ] No `.` allowed in session names (tmux target separator)
- [ ] All existing call sites use the consolidated function
- [ ] CLI `kill` subcommand validates session name before passing to tmux

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Security Sentinel, Pattern Recognition Specialist, Learnings Researcher
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** Fixed regex to [a-zA-Z0-9_-] (no period). Added consolidate sanitize_tmux_name task and validate_target() method to Phase 1. Updated security section to note validation at TmuxManager boundary.
