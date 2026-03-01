---
date: 2026-02-28
topic: embedded-session-viewer
---

# Embedded Session Interactor

## What We're Building

Replace the Session Radar with a **live, embedded, interactive Claude Code session** that
occupies the right side of nexus. This is not a viewer — it's a full session interactor.
Keystrokes flow to the active session **by default**. Nexus controls are accessed exclusively
via Alt+key modifiers, keeping zero conflicts with Claude Code or terminal keybindings.

When no tmux pane exists for the selected session (detached/dead/scanned), the panel shows
a **conversation log** parsed from the session's JSONL data.

## Layout

```
┌─ TOP BAR (3 lines) ─ local timezone date ────────────────┐
├─────────────┬────────────────────────────────────────────────┤
│ TREE (25%)  │ SESSION (interactive, ~5/6 height)             │
│             │ Live tmux pane OR conversation log              │
│             │                                                │
│             │ All keystrokes go here by default.             │
│             │ Full Claude Code interaction.                   │
│             ├────────────────────────────────────────────────┤
│             │ DETAIL (~1/6 height, compact metadata)         │
└─────────────┴────────────────────────────────────────────────┘
```

**Removed panels:** Session Radar, Activity Strip.
**Modified panels:** Top bar (date in user's local timezone).
**New panel:** Session Interactor (replaces radar).

## Interaction Model

### Always-on: no focus switching, no modes

The embedded session is the **primary input target**. All keystrokes (including Ctrl combos)
pass through to the tmux session. Nexus commands use **Alt+key** exclusively:

| Keybind    | Action                     |
|------------|----------------------------|
| Alt+j      | Tree: select next          |
| Alt+k      | Tree: select previous      |
| Alt+Enter  | Tree: expand/collapse group |
| Alt+n      | New session                |
| Alt+d      | Delete session             |
| Alt+r      | Rename session             |
| Alt+m      | Move session to group      |
| Alt+g      | New group                  |
| Alt+x      | Kill tmux session          |
| Alt+f      | Fullscreen tmux attach     |
| Alt+h      | Help overlay               |
| Alt+q      | Quit nexus                 |

**Why Alt?** Ctrl combos (Ctrl+C, Ctrl+D, Ctrl+K, Ctrl+R, etc.) are essential for terminal
and Claude Code interaction. Alt is almost never used by terminal apps, giving us a clean
namespace with zero conflicts.

**Fullscreen attach (Alt+f):** Suspends nexus and directly attaches to the tmux session for
deep focused work. Detach (Ctrl+Q or tmux prefix+d) returns to nexus.

### Image input

Image input to Claude Code must work through the embedded session:

- **Clipboard paste (image):** Nexus detects paste (Ctrl+V / bracketed paste), checks system
  clipboard for image data via `arboard` crate. If image found, saves to temp file
  (`/tmp/nexus-paste-XXXX.png`) and sends the file path to the session via `tmux send-keys`.
  Claude Code receives the path and processes the image.
- **Clipboard paste (text):** Forward via `tmux load-buffer` + `tmux paste-buffer` for proper
  multi-line/special character handling.
- **Drag-and-drop:** Terminal emulators (iTerm2, Kitty, etc.) drop file paths as text input.
  This flows naturally through the keystroke forwarding — no special handling needed.

### Conversation log mode (no tmux pane)

When the selected session has no running tmux pane, keystrokes have nowhere to go.
Non-Alt keystrokes are ignored. The panel shows a read-only conversation log parsed from
the session JSONL. Alt+j/k still navigates the tree.

## Why This Approach

### tmux-mediated terminal mirroring (chosen)

Uses `tmux capture-pane -t <session> -p -e` to read the session's terminal buffer with ANSI
escape sequences, and `tmux send-keys -t <session>` to forward keystrokes. Nexus acts as a
mirror/proxy — tmux handles all terminal emulation natively.

- Leverages existing tmux infrastructure (no new PTY management)
- ANSI → ratatui conversion via `ansi-to-tui` crate for full-color rendering
- Polling-based refresh: ~100-150ms for near-real-time feel
- All Ctrl combos pass through cleanly via `tmux send-keys`

### Alternatives considered

**PTY embedding:** Spawn a PTY connected to `tmux attach-session`, use `vte` to parse VT100
output into cell grid. Lower latency but significantly more complex — PTY lifecycle
management, new dependencies, raw I/O coordination with ratatui. Overkill when tmux already
handles terminal emulation.

**tmux nested panes:** Run nexus inside a tmux pane and use tmux's own splitting. Loses
ratatui control over layout and makes the UI dependent on external tmux configuration.

## Key Decisions

- **Session is always interactive:** No Tab/Enter to "enter" the session. Keystrokes go
  directly to the tmux pane at all times.
- **Alt for all nexus commands:** Zero conflicts with terminal/Claude Code keybindings.
- **Fullscreen escape hatch (Alt+f):** Suspend nexus, attach directly to tmux session for
  deep work. Return on detach.
- **Live rendering:** `tmux capture-pane -e` + `ansi-to-tui` for full ANSI color + styling.
- **Conversation log fallback:** Parse JSONL for sessions without a tmux pane.
- **Image input:** Clipboard images detected via `arboard`, saved to temp file, path sent to
  session. Text paste via `tmux load-buffer`/`paste-buffer`. Drag-and-drop flows naturally.
- **Polling cadence:** ~100-150ms refresh for the active session pane.
- **Width split:** 25% tree / 75% session+detail.
- **Height split (right):** ~5/6 session / ~1/6 detail.
- **Removals:** Radar (radar.rs, radar_state.rs), Activity strip (activity.rs),
  FocusPanel enum simplification.
- **Top bar:** Date in user's local timezone.

## Components Affected

### New
- `widgets/session_interactor.rs` — terminal mirror + conversation log + keystroke forwarding
- `ansi-to-tui` dependency — ANSI escape sequence → ratatui Text conversion
- `arboard` dependency — cross-platform clipboard access (text + image read)

### Removed
- `widgets/radar.rs` — radar canvas widget
- `widgets/radar_state.rs` — blip computation, sweep animation
- `widgets/activity.rs` — activity strip widget
- Activity strip rendering in `ui.rs`

### Modified
- `types.rs` — remove `FocusPanel` enum entirely (no focus switching needed), or simplify
- `ui.rs` — new 3-zone layout (top bar, tree+session+detail), remove radar/activity
- `app.rs` — Alt+key command routing, keystroke forwarding, fast polling, fullscreen attach
- `tmux.rs` — add `capture_pane()` and `send_keys()` methods
- `scanner.rs` — conversation-level JSONL parsing for log fallback (may extend existing)
- `theme.rs` — session interactor panel theming
- `Cargo.toml` — add `ansi-to-tui` and `arboard` dependencies

## Resolved Questions

- **Key mapping:** Full coverage. All keyboard input (printable chars, Enter, Backspace, Tab,
  Escape, arrows, Home/End, PgUp/PgDn, Ctrl+key combos, function keys, Delete, Insert),
  mouse events (clicks, scroll, drag), and bracketed paste — all forwarded to the tmux
  session. The embedded session must behave identically to a native terminal.
- **Conversation log depth:** Human + assistant turns only. Clean, readable conversation flow.
  Skip tool calls, system messages, and metadata.
- **Scroll behavior:** Native. All scroll input (mouse scroll, Up/Down, PgUp/PgDn) is
  forwarded directly to the tmux pane via `send-keys`. tmux and the session handle their
  own scrollback. No nexus-managed viewport or capture-pane offset tricks.
- **Group node selected:** Auto-select first child session. Groups are just organizers —
  landing on a group immediately selects its first session and shows that session's content.

## Next Steps

-> `/workflows:plan` for implementation details
