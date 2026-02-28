---
task: "09"
title: Layout Orchestration & Final Wiring
type: feat
date: 2026-02-28
status: pending
depends_on: ["01", "02", "03", "04", "05", "06", "07", "08"]
---

# 09 — Layout Orchestration & Final Wiring

## Goal

Compose all modules into the final Tactical Deck. This is the integration task — it connects the scanner, database, config, tmux, widgets, and theme into a working application.

## Scope

### Layout Composition

- Ratatui `Layout` splits matching the Tactical Deck wireframe:
  - Top bar: `Constraint::Length(3)`
  - Main area: `Constraint::Min(0)` split horizontally 50/50
  - Right column: split vertically ~50/50 (radar / detail)
  - Bottom strip: `Constraint::Length(3)`

### State Management

- Central `App` struct holding:
  - Session data (from scanner, cached in SQLite)
  - Group hierarchy (from config + database)
  - Selection state (shared between tree and radar)
  - Active tmux sessions (from tmux manager)
  - Current focus zone (tree, radar, detail)
- Tick-based update loop for radar animation and tmux polling

### Focus & Navigation

- `Tab` cycles focus between tree and radar
- Keybindings are context-sensitive (tree keys vs radar keys vs global)
- `Enter` on a session → tmux manager launches/resumes it
- `Ctrl+Q` from tmux → returns to Nexus TUI

### Startup Flow

1. Check single-instance lock
2. Validate tmux is available
3. Load TOML config (or create default)
4. Initialize SQLite database
5. Scan sessions, cache in SQLite, apply auto-grouping rules
6. Boot sequence animation (TachyonFX)
7. Render Tactical Deck

### Wiring

- Tree selection → updates detail panel + radar highlight
- Radar selection → updates detail panel + tree highlight
- Tmux state changes → update activity strip + session active indicators
- Group CRUD → persist to SQLite + update tree + update radar
- Config changes → re-apply auto-grouping rules

## Acceptance Criteria

- [ ] All 5 zones render correctly in the Tactical Deck layout
- [ ] Tree and radar stay in sync (bidirectional selection)
- [ ] Resuming a session opens Claude in a tmux pane
- [ ] Creating a new session in a group inherits the group's working directory
- [ ] Boot animation plays on startup
- [ ] `Ctrl+Q` returns from a Claude session to Nexus
- [ ] Graceful shutdown (close tmux socket, release lock, restore terminal)
- [ ] Works end-to-end against real Claude session data

## Notes

This task depends on all others. It's the integration and polish pass. By the time this starts, all individual modules should be functional with their own tests.
