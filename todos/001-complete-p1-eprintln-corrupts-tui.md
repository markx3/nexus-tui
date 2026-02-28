---
status: pending
priority: p1
issue_id: "001"
tags: [code-review, architecture, quality]
dependencies: []
---

# eprintln! Will Corrupt TUI When Scanner Is Wired Into App

## Problem Statement

The scanner module uses `eprintln!` in 4 locations (lines 229, 248, 285, 332) to emit warnings about I/O errors and malformed JSON. Once the scanner is called from within `App::run()` (which owns the ratatui terminal in alternate screen mode), any `eprintln!` output will write raw text into the alternate screen buffer, corrupting the TUI display.

This is a **blocker for integration** with downstream tasks 05-07 (session tree, radar, info panels).

## Findings

- **Security Sentinel:** Noted error messages may leak file content fragments via serde_json error Display output.
- **Architecture Strategist:** Flagged as the #1 P1 finding — "the single most important architectural issue."
- **Pattern Recognition:** Confirmed inconsistency between `?` propagation on `File::open` vs `eprintln!` on directory errors.
- **All agents agree:** This must be fixed before integration.

**Affected locations:**
- `src/scanner.rs:229` — unreadable projects dir
- `src/scanner.rs:248` — unreadable project subdir
- `src/scanner.rs:285` — parse failure
- `src/scanner.rs:332` — malformed JSON line

## Proposed Solutions

### Option A: Return warnings alongside results (Recommended)
```rust
pub struct ScanResult {
    pub sessions: Vec<SessionInfo>,
    pub warnings: Vec<String>,
}
```
- **Pros:** Pure function, no side effects, caller decides how to surface warnings, easy to test
- **Cons:** Changes the public API signature
- **Effort:** Small
- **Risk:** Low

### Option B: Collect into thread-local or passed-in Vec
Pass a `&mut Vec<String>` through the call chain for warnings.
- **Pros:** No public API change needed if kept internal
- **Cons:** Threading a mutable reference through every function is noisy
- **Effort:** Small
- **Risk:** Low

### Option C: Add `tracing` crate with file subscriber
- **Pros:** Industry standard, structured logging, configurable
- **Cons:** New dependency, more complex setup, overkill for current needs
- **Effort:** Medium
- **Risk:** Low

## Recommended Action

Option A — cleanest, most testable, aligns with the scanner's "pure data" philosophy.

## Technical Details

- **Affected files:** `src/scanner.rs`
- **Components:** `scan()`, `parse_session_file()` functions
- **Secondary benefit:** Also fixes the information disclosure concern (Finding 2 from security review) by giving the caller control over what to surface.

## Acceptance Criteria

- [ ] No `eprintln!` calls remain in `src/scanner.rs`
- [ ] Warnings are collected and returned to the caller
- [ ] Existing tests still pass
- [ ] serde_json error details are not exposed in warning messages (truncate to path + line number)

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from code review of task 02 | All 5 agents flagged this |

## Resources

- PR commit: `8c4ed00`
- Plan: `docs/plans/2026-02-28-feat-session-scanner-plan.md` (acknowledged eprintln as temporary)
