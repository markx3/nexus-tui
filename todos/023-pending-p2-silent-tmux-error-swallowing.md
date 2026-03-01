---
status: pending
priority: p2
issue_id: "023"
tags: [code-review, error-handling, ux]
dependencies: []
---

# Silent Tmux Error Swallowing — No User Feedback

## Problem Statement

Tmux launch/resume failures are silently discarded with `let _ =`:
- `app.rs:221` — `let _ = self.tmux.resume_session(&tmux_name);`
- `app.rs:223` — `let _ = self.tmux.launch_session(&tmux_name, &cwd);`

A user pressing Enter on a session gets zero feedback if tmux fails (wrong path, tmux not installed, permission denied, etc.).

Also in `main.rs`:
- `main.rs:38` — `let _ = db.create_group(...)` swallows non-duplicate errors
- `main.rs:57` — `let _ = tmux.setup_keybindings()` swallows setup failures

## Findings

- **Architecture Strategist:** Finding 4.3 (LOW) — "Store error as transient status message."
- **Pattern Recognition:** Finding 2.7 (LOW) — "user expects visible feedback."

## Proposed Solutions

### Option A: Add status_message to App (Recommended)
Add `pub status_message: Option<(String, Instant)>` to App. Display in activity strip or top bar. Auto-clear after 5 seconds.

- **Pros:** User sees errors, minimal UI change, self-clearing
- **Cons:** New field in App
- **Effort:** Small
- **Risk:** Low

### Option B: Log to activity feed
Push tmux errors into the activity stream.

- **Pros:** Persistent record
- **Cons:** Activity feed shows tmux windows, not errors — different data type
- **Effort:** Medium
- **Risk:** Low

## Recommended Action

Option A.

## Technical Details

**Affected files:** `src/app.rs`, `src/main.rs`

## Acceptance Criteria

- [ ] Tmux launch/resume errors surface in the UI
- [ ] Group creation errors (non-duplicate) are propagated
- [ ] Status message auto-clears after timeout

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Silent failures frustrate users |
