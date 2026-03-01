---
status: complete
priority: p1
issue_id: "034"
tags: [code-review, performance, architecture]
dependencies: []
---
# Move ANSI Parsing to Capture Worker Thread

## Problem Statement
The plan stores `SessionContent::Live(String)` (raw ANSI) and parses with `to_text()` in the render function. This means ANSI parsing runs on the main thread every frame when content changes (~12.5 times/second during active output). ANSI parsing competes with keystroke dispatch on the same thread. Also, if content-hash shows no change but we still re-render, we'd re-parse unchanged content.

## Findings
- `ansi-to-tui` parsing of a 200x50 terminal buffer takes ~200-500μs per call
- During active Claude Code streaming, content changes every poll cycle
- Main thread also handles keystroke forwarding (send-keys at 2-5ms each)
- Combined per-frame cost: 200μs parse + potential 5ms send-keys = 5.2ms of 16ms budget (32%)
- `to_text()` returns `Text<'_>` borrowing from input — fine for single-frame transient use
- But if parsing moves to worker thread, must use `into_text()` (owned) since `Text` must be Send
- Plan's `InteractorState` has no `current_content` field to hold last-received content for re-rendering
- Found by: Performance Oracle, Pattern Recognition Specialist

## Proposed Solutions

### Option 1: Parse in worker, send pre-parsed Text (Recommended)
**Approach:** Worker thread calls `into_text()` on raw capture, sends `CaptureResult::Parsed(Text<'static>)` over channel. Main thread only does try_recv + render.
**Pros:** Zero parsing overhead on main thread, clean separation of concerns
**Cons:** Uses `into_text()` (owned allocation) instead of `to_text()` (zero-copy), Text<'static> is larger to send over channel
**Effort:** 2 hours
**Risk:** Low

### Option 2: Cache parsed Text on main thread
**Approach:** Keep parsing on main thread but cache the parsed `Text` alongside the raw content hash. Only re-parse when hash changes.
**Pros:** Uses `to_text()` zero-copy, simpler worker thread
**Cons:** Parsing still happens on main thread (occasionally), need extra caching state
**Effort:** 1 hour
**Risk:** Low

### Option 3: Keep current design, profile later
**Approach:** Parse in render function as the plan specifies. Add caching only if profiling shows a problem.
**Pros:** Simplest initial implementation
**Cons:** Known performance cost on main thread, will likely need fixing
**Effort:** 0 (defer)
**Risk:** Medium — may cause visible lag during active sessions

## Technical Details
**Affected files:** capture worker (new), src/widgets/session_interactor.rs, session_interactor_state.rs
**Note:** If Option 1 is chosen, change `CaptureResult::Content(String)` to `CaptureResult::Parsed(Text<'static>)`
**Note:** InteractorState needs a `current_content: SessionContent` field to hold latest content for re-rendering (missing from plan)

## Acceptance Criteria
- [ ] ANSI parsing does not run on the main event loop thread during normal operation
- [ ] Content-hash diffing prevents unnecessary re-parsing
- [ ] InteractorState holds current content for frame re-rendering

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Performance Oracle, Pattern Recognition Specialist
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** Worker now calls into_text() and sends Text<'static> over channel. SessionContent::Live changed from String to Text<'static>. Added current_content field to InteractorState. Removed to_text() references from widget section.
