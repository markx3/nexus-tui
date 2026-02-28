---
task: "07"
title: Info Panels (Detail, Status Bar, Activity Strip)
type: feat
date: 2026-02-28
status: pending
depends_on: []
---

# 07 — Info Panels

## Goal

Three display-only panels that provide context: the top status bar, the bottom-right detail panel, and the bottom activity strip.

## Scope

### Top Status Bar (3 rows, full width)

- System info: `SYS:ONLINE`, total session count, active session count, system memory, current date
- Heavy border frame (`━━━`)
- Styled with neon separators (`══`) between metrics

### Detail Panel (bottom-right)

- Shows full metadata for the currently selected session:
  - Session name (slug or custom override)
  - Working directory
  - Model, git branch
  - Message count, token estimate
  - First user message as topic preview (truncated)
  - Sub-agent count with progress indicators if available
- Action keybindings displayed at bottom: `[R]esume [N]ew [D]elete [M]ove`
- Updates reactively when tree/radar selection changes

### Activity Strip (3 rows, full width, bottom)

- One inline gauge per active tmux session: `▶ session-name ████▒░░ status`
- Block-character progress bars (`▏▎▍▌▋▊▉█`)
- Shows "idle", "active", or percentage if determinable
- Scrolls horizontally if more active sessions than screen width allows

## Acceptance Criteria

- [ ] Status bar renders with live system stats
- [ ] Detail panel shows all metadata fields for selected session
- [ ] Detail panel updates when selection changes
- [ ] Activity strip shows gauges for active sessions
- [ ] All panels render correctly with mock data
- [ ] Graceful handling when no session is selected (empty/placeholder state)

## Notes

These are relatively simple display widgets. Develop with mock data, wire to real state later.
