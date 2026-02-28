---
task: "05"
title: Session Tree Widget
type: feat
date: 2026-02-28
status: pending
depends_on: []
---

# 05 — Session Tree Widget

## Goal

The left panel: a collapsible, navigable hierarchical tree showing groups and sessions. The primary interaction surface of Nexus.

## Scope

- Render a tree where nodes are groups (`◈`, `⬡`) and leaves are sessions
- Collapsible/expandable groups with `Enter` or `→`/`←`
- Vim-style navigation: `j`/`k` (up/down), `Enter` (expand/select), `Esc` (collapse)
- Visual indicators:
  - `▶` for active sessions (running in tmux)
  - Relative timestamp ("now", "2h", "1d", "3w")
  - Dimmed style for inactive/stale sessions
- Selection state: highlighted node drives the detail panel and radar highlight
- Group CRUD inline:
  - `n` — new group (inline text input)
  - `N` — new session in selected group
  - `r` — rename selected node
  - `m` — move selected session/group (enters move mode)
  - `d` — delete with confirmation
- Search/filter with `/` (fuzzy match on session name, topic, cwd)

## Acceptance Criteria

- [ ] Renders a hierarchical tree with proper indentation and icons
- [ ] Expand/collapse works on group nodes
- [ ] j/k navigation with visible cursor
- [ ] Active sessions show `▶` indicator
- [ ] Inline CRUD operations work (create group, rename, move, delete)
- [ ] Search filters the tree in real-time
- [ ] Works with mock data (no dependency on scanner or database)

## Notes

Can use `tui-tree-widget` crate or build a custom tree widget. Develop against a hardcoded mock tree first, wire to real data later.
