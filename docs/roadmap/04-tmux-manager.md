---
task: "04"
title: Tmux Manager
type: feat
date: 2026-02-28
status: pending
depends_on: []
---

# 04 — Tmux Manager

## Goal

Module that manages Claude Code sessions through tmux — launching, resuming, detecting active sessions, and providing custom keybindings for navigation.

## Scope

- Use a dedicated tmux socket (e.g., `tmux -L nexus`) to isolate from user's personal tmux sessions
- **Launch new session**: `claude` in a new tmux window within the Nexus socket, with a given working directory
- **Resume session**: `claude --resume <session-id>` in a new tmux window
- **Detect active sessions**: query tmux to determine which Claude sessions are currently running (match by session UUID in window name or command)
- **Custom keybinding**: `Ctrl+Q` detaches from the current Claude pane and returns focus to the Nexus TUI
- **List active windows**: return which tmux windows are alive (for the activity strip)
- **Kill session**: gracefully close a tmux window

## Acceptance Criteria

- [ ] Can launch a new Claude session in a tmux pane with a given cwd
- [ ] Can resume an existing session by UUID
- [ ] Correctly detects which sessions are active in tmux
- [ ] `Ctrl+Q` returns to the Nexus TUI (custom tmux keybind on the nexus socket)
- [ ] Validates tmux is installed on startup (clear error message if not)

## Notes

Pure library module. Interacts with tmux via CLI commands (`std::process::Command`). Can be tested independently by spawning/killing tmux windows.
