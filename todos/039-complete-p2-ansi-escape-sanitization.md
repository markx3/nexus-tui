---
status: complete
priority: p2
issue_id: "039"
tags: [code-review, security]
dependencies: []
---
# Sanitize Non-SGR ANSI Escape Sequences from Captured Pane

## Problem Statement
`tmux capture-pane -e` includes all ANSI escape sequences from the session. `ansi-to-tui` handles standard SGR (color/style) sequences but its behavior with non-SGR sequences (OSC, DCS, Sixel, etc.) is not verified. A malicious or compromised session could potentially manipulate the host terminal's clipboard (OSC 52), set window titles (OSC 0), or trigger terminal-specific behaviors.

## Findings
- OSC sequences `\x1b]` can: set window titles, manipulate clipboard (OSC 52), trigger hyperlinks
- DCS sequences can send arbitrary data to the terminal
- `ansi-to-tui` v8.0.1 handles SGR well but non-SGR behavior is undocumented
- If unrecognized sequences are passed through as raw bytes, they would be interpreted by the host terminal
- Found by: Security Sentinel (MEDIUM #4)

## Proposed Solutions

### Option 1: Strip non-SGR escapes before parsing (Recommended)
**Approach:** Add a sanitization pass that keeps only CSI SGR sequences (`\x1b[...m`) and strips OSC, DCS, Sixel, and other non-standard sequences before feeding to ansi-to-tui.
**Pros:** Defense-in-depth regardless of library behavior
**Cons:** Extra processing step (but fast for <10KB buffers)
**Effort:** 2-3 hours
**Risk:** Low

### Option 2: Verify ansi-to-tui behavior and trust it
**Approach:** Test that ansi-to-tui strips/ignores non-SGR sequences. If it does, no extra sanitization needed.
**Pros:** Less code
**Cons:** Relies on library behavior that could change between versions
**Effort:** 1 hour
**Risk:** Medium — library version upgrade could introduce vulnerability

## Technical Details
**Affected files:** capture worker thread, or a utility function in tmux.rs
**Test cases needed:** Feed OSC 52 (clipboard write), OSC 0 (title set), DCS sequences into parser, verify they don't appear in output

## Acceptance Criteria
- [ ] Non-SGR ANSI sequences from captured pane do not reach the host terminal
- [ ] Test verifies OSC 52 clipboard write sequence is stripped
- [ ] Test verifies OSC 0 title set sequence is stripped

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Security Sentinel
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** Added sanitize_ansi() task to Phase 2 with OSC/DCS/Sixel test cases. Worker pseudocode calls sanitize_ansi() before into_text(). Added NFR for non-SGR sequence isolation. Updated risk table.
