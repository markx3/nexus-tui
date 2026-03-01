---
title: "feat: Embedded Session Interactor"
type: feat
date: 2026-02-28
brainstorm: docs/brainstorms/2026-02-28-embedded-session-viewer-brainstorm.md
deepened: 2026-02-28
---

# feat: Embedded Session Interactor

## Enhancement Summary

**Deepened on:** 2026-02-28
**Revised on:** 2026-03-01 (incorporated 12 code review findings: 4 P1, 6 P2, 2 P3)
**Sections enhanced:** 7 (all major sections)
**Research agents used:** Performance Oracle, Security Sentinel, Architecture Strategist,
Code Simplicity Reviewer, Pattern Recognition Specialist, Best Practices Researcher,
Framework Docs Researcher, Agent-Native Reviewer

### Key Improvements
1. **Capture worker with ANSI parsing** — worker thread calls `into_text()` and sends
   pre-parsed `Text<'static>` over channel; main thread has zero parsing overhead
2. **Security hardening** — `SendKeysArgs` enum with `Literal`/`Named(&'static str)`;
   `sanitize_ansi()` strips non-SGR escapes; named tmux buffer for paste; session name
   validation at TmuxManager API boundary (no `.` — tmux separator)
3. **Fire-and-forget send-keys** — `Command::spawn()` avoids blocking the 16ms frame budget
4. **Adaptive capture polling** — 30ms active, 500ms idle backoff; blocking `recv()` when idle
5. **Input routing delegation** — InteractorState owns tmux input handling; App receives
   `RouteResult` enum (Forwarded/NexusCommand/Ignored)
6. **CLI agent parity** — `nexus send/capture/delete/rename/move/group` subcommands
7. **Phase consolidation** — 3 phases; FocusPanel removal in Phase 2 alongside radar deletion
8. **YAGNI simplifications** — no TmuxKeyName enum, no LRU cache, no SessionContent::Empty,
   no keystroke batching, no fxhash, no group auto-selection

### Key Design Decisions (Revised)
- `ansi-to-tui` 8.0.1 confirmed compatible with ratatui 0.30 via `ratatui-core ^0.1.0`
- Use `into_text()` (owned `Text<'static>`) in worker — must cross thread boundary
- Add `-N` flag to `capture-pane` for alternate screen mode
- Omit `-J` flag to preserve character grid alignment
- Debounce resize: skip capture on resize tick
- Non-SGR ANSI sequences stripped before parsing (OSC/DCS defense-in-depth)
- Named tmux buffer `nexus-paste` for paste operations (no TOCTOU race)
- Group node selection shows empty state (preserves group CRUD)

---

## Overview

Replace the Session Radar with a live, embedded, interactive Claude Code session occupying
the right panel of nexus. All keystrokes (keyboard, mouse, paste) flow to the tmux session
by default. Nexus commands are accessed exclusively via Alt+key modifiers. When no tmux pane
exists, the panel shows a read-only conversation log parsed from JSONL data.

This transforms nexus from a session launcher into a full session multiplexer — you can
interact with Claude Code sessions directly within nexus while maintaining tree navigation
and session management.

## Problem Statement / Motivation

Currently nexus shows a decorative Session Radar on the right side. To interact with a
Claude Code session, the user must press Enter to fullscreen-attach to the tmux session,
losing all nexus context. This creates a constant context-switching loop:
nexus → tmux attach → work → detach → nexus → navigate → attach again.

The embedded interactor eliminates this by putting the live session directly in the UI.

## Proposed Solution

**tmux-mediated terminal mirroring:** Use `tmux capture-pane -p -e -N` to read the session's
terminal buffer with ANSI escape sequences (including alternate screen), render it via
`ansi-to-tui` into ratatui widgets, and forward all user input via `tmux send-keys`. The
tmux pane is resized to match the interactor panel dimensions so content renders correctly.

### Research Insights

**Best Practices (tmux capture-pane):**
- Use `-p -e -N` flags: `-p` outputs to stdout, `-e` includes ANSI escapes, `-N` preserves
  alternate screen content (critical for Claude Code's TUI elements)
- Omit `-J` flag — it joins wrapped lines but destroys character grid alignment, causing
  column mismatches with ratatui's fixed-width rendering
- Resize pane *before* capture (not after) — capture after resize returns content formatted
  for the new size; skip capture on the same tick as resize to let tmux reflow

**Performance (capture efficiency):**
- Byte-equality compare capture output (`last_raw != current_raw`) before parsing
  with `ansi-to-tui` — idle sessions produce identical output frame after frame, saving ~80%
  of ANSI parsing CPU. No hashing dependency needed for <10KB payloads.
- Use `into_text()` (owned `Text<'static>`) in the worker thread since parsed `Text` must
  cross thread boundary via mpsc channel. Worker sends pre-parsed content to main thread.

**Security (tmux send-keys):**
- Create a `SendKeysArgs` enum: `Literal(String)` (always uses `-l` flag) vs
  `Named(&'static str)` where named keys are compile-time constants from match arms
  (Enter, BSpace, Up, Down, C-a through C-z, F1-F12, etc.)
- Never interpolate raw user strings into tmux commands without the `-l` flag
- Validate session names passed to `-t` against `[a-zA-Z0-9_-]` to prevent target injection
  (exclude `.` — tmux target separator for session:window.pane)
- Validation belongs at the TmuxManager API boundary (defense-in-depth), not at call sites

**Framework Compatibility:**
- `ansi-to-tui` 8.0.1 confirmed compatible with ratatui 0.30 via `ratatui-core ^0.1.0`
- `arboard` 3.6.1: macOS fully supported; `Clipboard` is neither Send nor Sync — create
  per-use on the main thread, do not store in App struct

## Technical Approach

### Architecture

```
┌─ TOP BAR (3 lines) ─ local timezone date ────────────────┐
├─────────────┬────────────────────────────────────────────────┤
│ TREE (25%)  │ SESSION INTERACTOR (~5/6 height)               │
│             │ Live terminal mirror OR conversation log        │
│             │                                                │
│             │ All input flows here by default.               │
│             │ Full Claude Code interaction.                   │
│             ├────────────────────────────────────────────────┤
│             │ DETAIL (~1/6 height, compact metadata)         │
└─────────────┴────────────────────────────────────────────────┘
```

**Data flow (live session):**
```
User input ──► nexus event loop ──► Alt key? ──► yes ──► nexus command
                                       │
                                       no
                                       │
                                       ▼
                              tmux send-keys -t <session>
                                       │
                                       ▼
                              tmux session processes input
                                       │
                              (next 100ms poll cycle)
                                       │
                                       ▼
                              tmux capture-pane -t <session> -p -e
                                       │
                                       ▼
                              ansi-to-tui parses ANSI → ratatui Text
                                       │
                                       ▼
                              render in session interactor panel
```

### Critical Design Decisions

#### Pane Geometry Synchronization

The tmux pane must be resized to match the interactor panel's inner dimensions. Without
this, `capture-pane` returns content formatted for the wrong size, producing garbled output.

- Use `tmux resize-pane -t <session> -x <cols> -y <rows>` with the interactor panel's
  inner dimensions (panel area minus borders)
- Resize on: session selection change, terminal resize events, initial display
- Claude Code will receive SIGWINCH and reflow — this is correct behavior
- Store last-resized dimensions to avoid redundant resize calls

#### Capture Worker Thread (Required from Day One)

~~Keep the event loop single-threaded and synchronous for v1.~~ **Revised:** A dedicated
capture worker thread is essential for responsive UX, not a v2 optimization.

**Why synchronous won't work:**
- `tmux capture-pane` spawns a subprocess (~2-5ms) — this blocks the event loop
- At 100ms polling + fast typing, the event loop spends 5-8% of time blocked
- But the *variance* is the problem: occasional 10-20ms subprocess stalls create perceptible
  keystroke-to-display lag, especially during bursts of typing
- Users comparing embedded session to native tmux will notice any added latency

**Implementation:**
```rust
// Capture worker thread — runs independently of event loop
// Receives a cloned TmuxManager (TmuxManager derives Clone; only holds socket_name: String)
fn capture_worker(
    tmux: TmuxManager,  // obtained via tmux_manager.clone()
    session_rx: mpsc::Receiver<String>,   // session name to capture
    content_tx: mpsc::Sender<Option<Text<'static>>>,
) {
    let mut current_session = String::new();
    let mut last_raw: Vec<u8> = Vec::new();
    let mut poll_interval = Duration::from_millis(30);
    loop {
        // Check for session change (non-blocking)
        if let Ok(new_session) = session_rx.try_recv() {
            current_session = new_session;
            last_raw.clear(); // force re-parse on session switch
            poll_interval = Duration::from_millis(30);
        }
        if current_session.is_empty() {
            // Block until a session is selected — no CPU burn
            if let Ok(new_session) = session_rx.recv() {
                current_session = new_session;
                last_raw.clear();
                poll_interval = Duration::from_millis(30);
            }
            continue;
        }
        // Capture
        match tmux.capture_pane(&current_session) {
            Ok(raw) => {
                let raw_bytes = raw.as_bytes();
                if raw_bytes != last_raw.as_slice() {
                    last_raw = raw_bytes.to_vec();
                    // Parse ANSI on the worker thread — main thread receives ready-to-render Text
                    let sanitized = sanitize_ansi(raw_bytes);
                    let parsed: Text<'static> = sanitized.into_text().unwrap_or_default();
                    let _ = content_tx.send(Some(parsed));
                    poll_interval = Duration::from_millis(30); // active: fast polling
                } else {
                    // Content unchanged — back off
                    poll_interval = (poll_interval * 2).min(Duration::from_millis(500));
                }
            }
            Err(_) => {
                // Session gone — existing tmux poll handles death detection
                let _ = content_tx.send(None);
            }
        }
        std::thread::sleep(poll_interval);
    }
}
```

- Main event loop: `content_rx.try_recv()` to pick up new pre-parsed `Text<'static>` (non-blocking)
- On session selection change: send new session name via `session_tx`
- Byte-equality comparison and ANSI parsing both happen in the worker thread — main thread only receives ready-to-render Text
- `send-keys` calls use `Command::spawn()` (fire-and-forget) to avoid blocking the main event loop. Session name is validated before the call. Capture worker's death detection handles session death.

#### Keystroke-to-tmux Mapping

Map crossterm `KeyEvent` to tmux `send-keys` arguments using a type-safe `SendKeysArgs` enum:

```rust
/// Type-safe tmux send-keys arguments — prevents command injection by construction
#[derive(Debug, Clone)]
enum SendKeysArgs {
    /// Literal text — always sent with `-l` flag. Safe for any user input.
    Literal(String),
    /// Named tmux key — compile-time constant from match arms on KeyCode.
    /// Injection-safe because values are &'static str from the key_event_to_send_args match.
    Named(&'static str),
}
```

- Printable characters → `SendKeysArgs::Literal(char.to_string())`
- Special keys → `SendKeysArgs::Named("Enter")`, `SendKeysArgs::Named("BSpace")`, etc.
- Ctrl+key → `SendKeysArgs::Named("C-c")` etc. (only alpha chars — validated by KeyCode match)
- Function keys → handled by match arm that maps F(1)..=F(12) to "F1".."F12"
- Mouse events → deferred to follow-up (see Deferred Complexity below)

**Key mapping belongs in `tmux.rs`**, not a separate module — it's tmux-specific logic
and keeping it with the tmux methods maintains cohesion.

#### Paste Handling

**v1 (this plan):** Text paste only via `tmux load-buffer` + `paste-buffer`.

Flow on paste event detection:
1. crossterm emits `Event::Paste(text)` (bracketed paste) or user presses Ctrl+V
2. If `Event::Paste(text)`:
   - Enforce size limit: reject paste >1MB with status message (prevents OOM from clipboard bombs)
   - Use `tmux load-buffer -b nexus-paste -` (named buffer, stdin) + `tmux paste-buffer -b nexus-paste -t <session> -d` (auto-cleanup)
   - `load_buffer_and_paste()` is a separate TmuxManager method (not bundled in send_keys)
3. If regular Ctrl+V (no bracketed paste): forward as keystroke `send-keys -t <session> C-v`

**Deferred to follow-up:** Image clipboard detection via `arboard`. Text paste covers 90% of use cases. See Deferred Complexity section.

#### Input Mode State Machine

```
InputMode::Normal (default)
├── All non-Alt keystrokes → tmux send-keys (forwarded to session)
├── All mouse events → tmux send-keys (forwarded to session)
├── Alt+key → nexus command dispatch
├── Alt+n → InputMode::TextInput (session name)
├── Alt+r → InputMode::TextInput (rename)
├── Alt+d → InputMode::Confirm (delete)
├── Alt+m → InputMode::GroupPicker (move)
└── Paste events → arboard check → tmux load-buffer/paste-buffer

InputMode::TextInput / Confirm / GroupPicker (modal overlays)
├── Keystrokes → nexus input buffer (NOT forwarded to tmux)
├── Enter/Esc → return to InputMode::Normal
├── Alt+key nexus commands → blocked during overlays
└── Capture-pane polling continues (display stays live behind overlay)
```

#### Conversation Log Mode

When the selected session has no running tmux pane:
- Parse JSONL for human + assistant turns only (skip tool calls, metadata)
- Render as a scrollable readonly view
- Arrow keys, PgUp/PgDn, and mouse scroll work for log navigation (exception
  to the "non-Alt keystrokes ignored" rule — no tmux pane to conflict with)
- Store parsed conversation for current session only; re-parse on session switch (sub-millisecond for 100 turns — no LRU needed)
- Limit to last 100 turns
- If no JSONL data exists: show centered "No session data available" message

**Simplification note:** For v1, conversation log can be minimal — just render the text
content of human/assistant turns with role labels and basic styling. Rich formatting,
code block highlighting, and turn collapsing are follow-up improvements. The primary
value is showing *something* for dead/detached sessions rather than a blank panel.

#### Group Node Selection

When the tree cursor lands on a group node, show an empty interactor with "Select a session"
message. No auto-advance to first child. This preserves the ability to select group nodes
for rename/delete operations. Group CRUD operations work as before.

### Deferred Complexity

The following features are explicitly **not** in this plan. They will be addressed in
follow-up plans after the core interactor is working end-to-end:

1. **Image paste via `arboard`** — clipboard detection, temp file management, macOS
   notification banner. Text paste covers 90% of use cases.
2. **Mouse event forwarding** — coordinate translation, hit-testing panel boundaries,
   mouse capture toggle. Keyboard-only interaction works for v1.
3. **Rich conversation log** — code block highlighting, turn collapsing, markdown rendering.
   Basic text with role labels is sufficient for the fallback view.
4. **Async/tokio** — the capture worker thread with `std::sync::mpsc` provides sufficient
   concurrency without adding a runtime dependency.

### Implementation Phases (Consolidated)

#### Phase 1: Foundation — Dependencies, Types, Tmux Methods, Key Mapping

Add new dependencies, extend the tmux layer, build the key mapping, and update types.
No UI changes yet — everything is testable in isolation.

**Tasks:**
- [ ] Add `ansi-to-tui = "8"` to `Cargo.toml` (do NOT add `arboard` — deferred)
- [ ] Add tmux methods to `src/tmux.rs` (4 methods):
  - `capture_pane(session_name: &str) -> Result<String>` — runs `tmux -L <socket> capture-pane -t <name> -p -e -N`
  - `send_keys(session_name: &str, args: &SendKeysArgs) -> Result<()>` — type-safe dispatch:
    `Literal` args use `-l` flag, `Named` args use key name from allowlist. Uses `Command::spawn()` (fire-and-forget).
  - `resize_pane(session_name: &str, cols: u16, rows: u16) -> Result<()>` — runs `resize-pane -t <name> -x <cols> -y <rows>`
  - `load_buffer_and_paste(session_name: &str, text: &str) -> Result<()>` — uses named buffer
    `nexus-paste`: `tmux load-buffer -b nexus-paste -` then `tmux paste-buffer -b nexus-paste -t <name> -d`
- [ ] Add `#[derive(Clone)]` to `TmuxManager` (capture worker needs its own instance)
- [ ] Add `SendKeysArgs` enum to `src/tmux.rs` (Literal/Named with `&'static str` — no separate TmuxKeyName enum needed)
- [ ] Consolidate duplicate `sanitize_tmux_name()` from `src/app.rs:804` and `src/main.rs:185` into `src/tmux.rs`
- [ ] Add `validate_target()` method to TmuxManager — called at API boundary in every method that takes a session name
- [ ] Add key mapping function to `src/tmux.rs`:
  - `fn key_event_to_send_args(event: &KeyEvent) -> Option<SendKeysArgs>` — maps crossterm
    KeyCode + KeyModifiers to SendKeysArgs
  - Handle all KeyCode variants: Char, Enter, Backspace, Tab, Escape, arrows, Home/End,
    PageUp/PageDown, Delete, Insert, F(1-12)
  - Handle Ctrl+key → `SendKeysArgs::Named("C-a")` etc. (only alpha chars from match arms)
  - F(u8) range validated by match: only F(1)..=F(12) produce Named values
  - Validate session name against `[a-zA-Z0-9_-]` regex before passing to `-t` (no `.` — tmux separator)
- [ ] Update `src/types.rs`:
  - Replace `PanelType::Radar` with `PanelType::SessionInteractor`
  - Remove `PanelType::ActivityStrip`
  - Remove radar-specific `ThemeElement` variants, add interactor variants
  - Add `SessionContent` enum: `Live(Text<'static>)` (pre-parsed by worker), `ConversationLog(Vec<ConversationTurn>)`
  - Add `ConversationTurn { role: Role, content: String }`
  - Add `pub enum Role { Human, Assistant }`
  - Keep `focused: bool` parameter pattern for signature consistency (always pass `false` for detail panel)
- [ ] Update `src/theme.rs`:
  - Add `SessionInteractor` panel border set (PLAIN, matching tree)
  - Update `style_for()`, `border_for()`, `border_style_for()` for new panel types
  - Reduce `fx_boot()` from 5 effects to 3 (top_bar, tree, right_column)
- [ ] Comprehensive unit tests:
  - All key mapping variants (printable, special, Ctrl+A-Z, F1-F12, unicode)
  - `SendKeysArgs` Literal/Named → tmux command string generation
  - Session name validation (valid names, injection attempts)
  - tmux methods (mark `#[ignore]` for CI, require real tmux)
  - Small-area guard: test that widgets don't panic on 0×0 or 1×1 areas

**Success criteria:** `cargo build` succeeds, all unit tests pass, tmux methods callable.

---

#### Phase 2: Widget + Layout — Rendering, Layout Rewrite, File Deletion

Create the interactor widget, rewrite the layout, delete radar/activity. At the end of
this phase, nexus displays the new layout with captured pane content (but no input
forwarding yet — the capture worker is wired up for display only).

**Tasks:**
- [ ] Create `src/widgets/interactor.rs`:
  - `pub fn render_interactor(frame, area, content: &SessionContent, focused: bool)`
    following existing widget pattern (keep `focused: bool` for consistency)
  - Block construction with theme integration (title shows session name, borders, focus style)
  - For `SessionContent::Live(text)`: render pre-parsed `Text<'static>` in a `Paragraph` widget
    (no ANSI parsing here — worker thread sends ready-to-render Text)
  - For `SessionContent::ConversationLog(turns)`: render with role labels and basic styling
  - When content is empty (no session selected or group node): show centered "Select a session" message
  - **Small-area guard**: if area is <10 cols or <3 rows, render nothing (don't panic)
- [ ] Create `src/widgets/interactor_state.rs`:
  - `InteractorState` struct holding:
    - `tmux: TmuxManager` (cloned instance for send_keys, resize, paste)
    - `content_rx: mpsc::Receiver<Option<Text<'static>>>` (from capture worker)
    - `session_tx: mpsc::Sender<String>` (to tell worker which session to capture)
    - `current_content: SessionContent` (holds latest content for re-rendering)
    - `last_resize: (u16, u16)` (to avoid redundant resize calls)
    - `current_conversation: Option<Vec<ConversationTurn>>` (current session only, re-parse on switch)
    - `log_scroll_offset: u16` (for conversation log scrolling)
  - `fn poll_content(&mut self) -> bool` — non-blocking try_recv, updates current_content, returns true if changed
  - `fn switch_session(&mut self, session_name: &str)` — sends to capture worker
  - `fn route_event(&mut self, event: Event) -> RouteResult` — input routing (see Phase 3)
  - `RouteResult` enum defined here (internal implementation detail)
- [ ] Add conversation JSONL parsing to `src/conversation.rs` (new module):
  - `fn parse_conversation(jsonl_path: &Path, max_turns: usize) -> Vec<ConversationTurn>`
  - Parse only `type: "human"` and `type: "assistant"` entries
  - Limit to last 100 turns
  - Add `jsonl_path: Option<PathBuf>` to `SessionSummary` — populated from DB (derived from
    session CWD via `~/.claude/projects/` convention)
- [ ] Rewrite `src/ui.rs` `draw()` function:
  - Top-level vertical split: `[top_bar: Length(3)]` + `[main_area: Fill(1)]`
  - Main area horizontal split: `[tree: Percentage(25)]` + `[right_column: Percentage(75)]`
  - Right column vertical split: `[interactor: Percentage(83)]` + `[detail: Percentage(17)]`
  - Extract layout computation to a shared function for resize calculations
  - Call `render_interactor()` instead of `render_radar()`
  - Remove `render_activity_strip()` call and `collect_tmux_names()` helper
  - Keep `render_detail()` as a separate call (don't merge into interactor)
- [ ] Update `render_top_bar()` in `src/widgets/top_bar.rs`:
  - Display date in user's local timezone (use existing `src/time_utils.rs` — no chrono dependency)
- [ ] Update `render_detail()` in `src/widgets/detail.rs`:
  - Compact layout for ~1/6 height (reduce padding, tighter formatting)
  - Show only: name, cwd, status, tmux name — one or two lines
- [ ] Delete `src/widgets/radar.rs`, `src/widgets/radar_state.rs`, `src/widgets/activity.rs`
- [ ] Remove `FocusPanel` enum, `SelectionState.focused_panel`, Tab key handler, `handle_radar_key()`, `advance_sweep()` call, `radar_state.compute_blips()`, `radar_state.select_by_session_id()`, and ui.rs filter logic at lines 81-88
- [ ] Add `sanitize_ansi(raw: &[u8]) -> Vec<u8>` function that strips non-SGR escape sequences
  (OSC, DCS, Sixel) before `into_text()` parsing in capture worker. Test with known-dangerous
  sequences: OSC 52 (clipboard write), OSC 0 (title set), DCS
- [ ] Update `src/widgets/mod.rs`: add new modules, remove deleted ones
- [ ] Remove `RadarState` from `App` struct, add `InteractorState`
- [ ] Spawn capture worker thread in `App::new()` or startup
- [ ] Update `fx_boot()` in `src/theme.rs` to produce 3 effects instead of 5
- [ ] Add render tests using `TestBackend`:
  - Live session rendering (pre-parsed Text, empty Text, various terminal sizes)
  - Conversation log (0, 1, 100 turns)
  - Empty/group-selected state ("Select a session" message)
  - Small-area guard (0×0, 1×1)
  - ANSI sanitization: OSC 52, OSC 0, DCS sequences stripped

**Success criteria:** `cargo build` succeeds, UI renders new layout, captured pane content
displays in real-time. No references to radar or activity remain. All render tests pass.

---

#### Phase 3: Event Loop + Polish — Alt Routing, Keystroke Forwarding, Fullscreen, Tests

The core behavioral change: all input flows to tmux by default, Alt intercepts for nexus.
Plus fullscreen attach, paste, cleanup, and comprehensive testing.

**Tasks:**
- [x] Enable bracketed paste in `src/main.rs` / `src/app.rs` terminal setup:
  - `crossterm::event::EnableBracketedPaste` (mouse capture deferred)
- [x] Rewrite `App::handle_events()` in `src/app.rs` — delegation pattern:
  - Drain all pending events per frame: `while event::poll(Duration::ZERO)? { ... }`
  - Poll `interactor_state.poll_content()` each frame for display updates
  - For `Event::Key` / `Event::Paste` / `Event::Resize`:
    Call `interactor_state.route_event(event)` → `RouteResult`
    - `RouteResult::Forwarded` → done (InteractorState handled it via tmux)
    - `RouteResult::NexusCommand(cmd)` → App dispatches cmd (Alt+j/k/n/d/etc.)
    - `RouteResult::Ignored` → drop (modal overlay active, non-Alt key)
  - App does NOT directly call `tmux.send_keys()` or `tmux.load_buffer_and_paste()`
- [x] Implement `route_event()` and `RouteResult` enum in `interactor_state.rs`:
  ```rust
  enum RouteResult {
      Forwarded,                    // Sent to tmux
      NexusCommand(NexusCommand),   // Alt+key, App should handle
      Ignored,                      // Modal overlay, key dropped
  }
  ```
- [x] InteractorState owns: key classification (Alt vs forward), tmux send_keys dispatch,
  paste validation + load_buffer_and_paste, resize debounce, capture polling. Holds a cloned
  TmuxManager.
  - Handle `Event::Paste(text)`: enforce 1MB size limit, use `tmux.load_buffer_and_paste()`
  - Handle `Event::Resize`: debounce (set resize_pending flag, skip capture on this tick),
    compute new interactor panel dimensions, call `tmux.resize_pane()` on next tick
  - Call `tmux.resize_pane()` on session selection change (if dimensions differ)
- [x] Rewrite nexus command dispatch (Alt-based):
  - `Alt+j` → `tree_state.move_cursor_down()` + `interactor_state.switch_session()`
  - `Alt+k` → `tree_state.move_cursor_up()` + `interactor_state.switch_session()`
  - `Alt+Enter` → toggle group expand/collapse
  - `Alt+n` → enter TextInput mode for new session
  - `Alt+d` → enter Confirm mode for delete
  - `Alt+r` → enter TextInput mode for rename
  - `Alt+m` → enter GroupPicker mode
  - `Alt+g` → enter TextInput mode for new group
  - `Alt+x` → kill tmux session (with confirm)
  - `Alt+f` → fullscreen attach (below)
  - `Alt+h` → toggle help overlay
  - `Alt+q` → quit nexus
- [x] **CLI Parity** — add subcommands so agents can use all new capabilities:
  - `nexus send <session> <text>` — calls `tmux.send_keys()` with Literal
  - `nexus capture <session> [--strip]` — calls `tmux.capture_pane()`, outputs raw or stripped
  - `nexus delete <session>` — calls `db.delete_session()`
  - `nexus rename <session> <name>` — calls `db.update_session_name()`
  - `nexus move <session> --group <group>` — calls `db.move_session_to_group()`
  - `nexus group create <name>` — calls `db.create_group()`
  - All support `--json` flag for machine-readable output
- [x] Implement session-death detection:
  - Capture worker sends `None` on error (session gone)
  - Existing 2-second tmux poll (`reconcile_tmux_state`) detects session death
  - Main loop transitions to conversation log mode when reconciliation marks session dead
- [x] Handle group node selection:
  - When tree cursor lands on a group node, show empty interactor with "Select a session" message
  - Group rename/delete operations work as before (cursor stays on group node)
- [x] Implement Alt+f fullscreen attach:
  - Reuse existing `attach_tmux_session()` pattern
  - Only available when selected session has active tmux pane
  - On return: restore terminal state, force full redraw
  - **Note:** document that on macOS, terminal must have "Use Option as Meta key" enabled
    for Alt keybindings to work (add to help overlay)
- [x] Update help overlay:
  - Show all Alt+key bindings
  - Add macOS Alt/Option note
  - Remove old keybindings (j/k, Tab, Enter for tree)
- [x] Integration tests (marked `#[ignore]`):
  - `test_capture_pane_returns_content`
  - `test_send_keys_reaches_session`
  - `test_resize_pane`
- [x] Update `src/mock.rs`:
  - Add `mock_conversation_turns()`, `mock_ansi_content()`
  - Remove `mock_tmux_sessions()` if no longer needed
- [x] Clean up:
  - `cargo clippy` — fix all warnings
  - Remove all dead radar/activity code paths
  - Verify `cargo test` — all existing + new tests pass
- [x] Manual testing:
  - Launch nexus, create session, type in embedded Claude Code
  - Navigate tree with Alt+j/k, verify session switches live
  - Paste text via Ctrl+V
  - Alt+f fullscreen attach and return
  - Resize terminal, verify pane adapts
  - Kill session externally, verify graceful transition to log
  - Select dead/scanned session, verify conversation log renders

**Success criteria:** All input forwarded correctly. Alt commands work. Fullscreen attach
works. All tests pass. No clippy warnings. Manual end-to-end verification complete.

## Alternative Approaches Considered

**PTY embedding:** Spawn a PTY connected to `tmux attach-session`, use `vte` to parse VT100
output. Lower latency but significantly more complex — PTY lifecycle, `vte` state machine,
raw I/O coordination. Overkill when tmux already handles terminal emulation.

**tmux nested panes:** Run nexus inside a tmux pane and use tmux's own splitting. Loses
ratatui control over layout and makes the UI dependent on external tmux configuration.

**Fully async event loop (tokio):** Use tokio for all subprocess calls. Adds a heavyweight
runtime dependency for subprocess calls that complete in 2-5ms. The capture worker thread
with `std::sync::mpsc` provides sufficient concurrency without tokio.

**Separate `key_map.rs` module:** Isolating key mapping in its own module. Rejected because
the mapping is inherently tmux-specific — it maps *to tmux arguments*, not generic key
representations. Keeping it in `tmux.rs` maintains cohesion.

## Acceptance Criteria

### Functional Requirements

- [ ] All keystrokes (printable, special, Ctrl combos, function keys) forwarded to active tmux session
- [ ] Bracketed paste forwarded via tmux load-buffer + paste-buffer (with 1MB size limit)
- [ ] Alt+j/k navigates session tree
- [ ] Alt+n/d/r/m/g/x perform session CRUD operations
- [ ] Alt+f suspends nexus and fullscreen-attaches to tmux session
- [ ] Alt+h shows help overlay with current keybindings (including macOS Alt/Option note)
- [ ] Alt+q quits nexus
- [ ] Conversation log displayed for sessions without tmux pane
- [ ] Conversation log scrollable with arrow keys
- [ ] Group nodes show empty interactor with "Select a session" message
- [ ] Every Alt+key TUI operation has a corresponding CLI subcommand
- [ ] Top bar displays date in user's local timezone
- [ ] Pane geometry synchronized with interactor panel dimensions
- [ ] Session death detected gracefully, transitions to conversation log

### Deferred (Follow-up Plan)
- [ ] Mouse events (click, scroll, drag) forwarded to active tmux session
- [ ] Image paste detected via arboard, saved to temp file, path sent to session
- [ ] File drag-and-drop works (file paths forwarded as text)
- [ ] Rich conversation log with code highlighting and turn collapsing

### Non-Functional Requirements

- [ ] Capture worker thread keeps main event loop non-blocking
- [ ] Byte-equality diffing in worker thread skips ANSI re-parsing on unchanged content
- [ ] Full ANSI color + styling rendered correctly (bold, underline, 256-color, RGB)
- [ ] Terminal resize handled correctly (debounced, pane resized, display reflows)
- [ ] No tmux command injection possible — `SendKeysArgs` enum enforces safety by construction
- [ ] Session names validated against `[a-zA-Z0-9_-]` at TmuxManager API boundary (no `.` — tmux separator)
- [ ] Non-SGR ANSI sequences from captured pane do not reach the host terminal

### Quality Gates

- [ ] `cargo build` succeeds with no warnings
- [ ] `cargo clippy` clean
- [ ] `cargo test` — all tests pass (existing + new)
- [ ] No references to radar, activity strip, or FocusPanel remain in codebase
- [ ] Manual end-to-end test of core user flows

## Dependencies & Prerequisites

- `ansi-to-tui = "8"` — ANSI escape sequence → ratatui Text conversion (v8.0.1 compatible
  with ratatui 0.30 via `ratatui-core ^0.1.0`)
- tmux must be installed (existing requirement)
- Bracketed paste support requires a modern terminal emulator (iTerm2, kitty, WezTerm, etc.)
- **macOS terminal configuration:** Alt/Option keybindings require "Use Option as Meta key"
  (iTerm2) or equivalent setting. Without this, crossterm receives composed characters
  instead of Alt+key events. Must be documented prominently.

## Risk Analysis & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| `ansi-to-tui` incompatible with ratatui 0.30 | Blocks Phase 2 | **Confirmed compatible** via ratatui-core ^0.1.0 — risk eliminated |
| Pane resize causes Claude Code visual glitches | Visual bugs | Debounce resize: skip capture on resize tick, let Claude Code settle after SIGWINCH |
| Complex ANSI output (nested TUI) parsing fails | Garbled display | `ansi-to-tui` is battle-tested; add fallback to strip ANSI |
| tmux send-keys key name mapping incomplete | Missing keys | Start with comprehensive match arms in `key_event_to_send_args`; add missing keys as found |
| macOS Alt/Option not configured | Alt commands don't work | Prominent note in help overlay + README; detect and show warning on startup if possible |
| Capture worker thread panics | Session display freezes | Use `catch_unwind` or monitor thread handle; restart on panic |
| App struct becomes god object (18+ fields) | Maintainability | Extract `InteractorState` as coordinator; keep capture/render/input concerns separated |
| ANSI escape injection from captured pane | Potential XSS-like | `sanitize_ansi()` strips non-SGR sequences before `ansi-to-tui` parsing — defense-in-depth |

## References & Research

### Internal References
- Brainstorm: `docs/brainstorms/2026-02-28-embedded-session-viewer-brainstorm.md`
- Event loop: `src/app.rs:100-146`
- Tmux integration: `src/tmux.rs:12-140`
- Widget pattern: `src/widgets/tree.rs:27` (render function signature)
- State pattern: `src/widgets/tree_state.rs:1-419`
- Theme integration: `src/widgets/tree.rs:34-45` (block construction)
- Fullscreen attach: `src/app.rs:640-658`
- Layout composition: `src/ui.rs:32-52`
- Types/enums: `src/types.rs:27-31` (FocusPanel), `src/types.rs:97-103` (InputMode)
- Session data: `src/db.rs` (scanner was removed; sessions loaded from DB)

### External References
- `ansi-to-tui` 8.0.1: https://crates.io/crates/ansi-to-tui — uses `IntoText` trait; `into_text()` for owned `Text<'static>` (worker thread)
- `arboard` 3.6.1 (deferred): https://crates.io/crates/arboard — Clipboard not Send/Sync, macOS shows notification banner
- tmux `capture-pane` flags: `-p` stdout, `-e` ANSI escapes, `-N` alternate screen (use all three; omit `-J`)
- tmux `send-keys` flags: `-l` literal text (always use for user input), `-H` hex (avoid)
- tmux `resize-pane`: `resize-pane [-DLMRU] [-t target-pane] [-x width] [-y height]`
- crossterm 0.29 event types: `Event::Key`, `Event::Mouse`, `Event::Paste`, `Event::Resize`
- crossterm 0.29 Alt detection: requires terminal "Use Option as Meta key" on macOS
- ratatui 0.30 breaking changes: `Alignment` → `HorizontalAlignment`, `block::Title` removed, `patch_style` now fluent

### Research Agents Consulted
- **Performance Oracle:** Capture thread, content-diff, adaptive polling, fire-and-forget send-keys, resize debounce
- **Security Sentinel:** SendKeysArgs injection prevention, session name validation, temp file safety
- **Architecture Strategist:** InteractorState extraction, input routing delegation, layout computation
- **Code Simplicity Reviewer:** Phase consolidation (7→3), deferred complexity, method count reduction
- **Pattern Recognition Specialist:** focused param, type placement, small-area guard, naming conventions
- **Best Practices Researcher:** tmux flags (-p -e -N, omit -J), named paste buffer, ANSI sanitization
- **Framework Docs Researcher:** ratatui 0.30 compat, crossterm 0.29 Alt handling, ansi-to-tui 8.0.1, arboard 3.6.1
