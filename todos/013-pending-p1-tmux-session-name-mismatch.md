---
status: pending
priority: p1
issue_id: "013"
tags: [code-review, correctness, security]
dependencies: []
---

# Tmux Session Name Identity Mismatch

## Problem Statement

`sanitize_tmux_name()` truncates session IDs to the last 8 characters and replaces non-alphanumeric chars with dashes. This creates two bugs:

1. **Collision risk:** Multiple session UUIDs can map to the same 8-char tmux name.
2. **`already_running` never matches:** The check compares the full `session_id` against `tmux_windows[].session_id`, but tmux windows are named with the sanitized 8-char form. The comparison always fails, causing duplicate tmux sessions to spawn instead of resuming existing ones.

## Findings

- **Security Sentinel:** Finding #1 (MEDIUM) — "both a functional bug and the highest-severity security finding."
- **Agent-Native Reviewer:** Finding 4 — noted `kill_window` exists but is never reachable.

**Affected locations:**
- `src/app.rs:312-319` — `sanitize_tmux_name()` truncates to 8 chars
- `src/app.rs:212-224` — `try_launch_session()` compares full ID vs sanitized name
- `src/app.rs:82-84` — `tmux_windows` populated from tmux's sanitized names

## Proposed Solutions

### Option A: Use full session ID with sanitization (Recommended)
Keep the full UUID but replace non-alphanumeric chars with dashes. No truncation.

- **Pros:** No collisions, simple identity mapping, tmux supports long session names
- **Cons:** Long tmux session names in `tmux ls` output
- **Effort:** Small
- **Risk:** Low

### Option B: Maintain a bidirectional ID-to-name mapping
Store `HashMap<SessionId, String>` mapping full IDs to sanitized tmux names.

- **Pros:** Allows any naming scheme, supports short names
- **Cons:** More state to maintain, mapping must survive restarts
- **Effort:** Medium
- **Risk:** Medium

### Option C: Store full session ID as tmux environment variable
Set `tmux set-environment -t <name> NEXUS_SESSION_ID <full-id>` on launch, read it back when listing.

- **Pros:** Short tmux names, reliable mapping
- **Cons:** More tmux commands, more complex parsing
- **Effort:** Medium
- **Risk:** Medium

## Recommended Action

Option A — simplest fix that eliminates both bugs.

## Technical Details

**Affected files:** `src/app.rs`
**Components:** App event handling, tmux integration
**Database changes:** None

## Acceptance Criteria

- [ ] `sanitize_tmux_name` does not truncate — uses full session ID (sanitized)
- [ ] `already_running` check correctly identifies running tmux sessions
- [ ] Pressing Enter on a running session resumes it (not spawns duplicate)
- [ ] No session name collisions with 1000+ sessions

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Identity mismatch between session ID domains |
