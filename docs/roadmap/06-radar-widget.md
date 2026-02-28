---
task: "06"
title: Interactive Radar Widget
type: feat
date: 2026-02-28
status: pending
depends_on: []
---

# 06 — Interactive Radar Widget

## Goal

The top-right panel: an interactive Braille-rendered radar that visualizes session groups as spatial clusters. The signature visual element of Nexus.

## Scope

### Rendering (Canvas + Braille markers)

- Concentric range rings (dim cyan) — represent time horizons (today, this week, this month, older)
- Crosshair at center (`⊕`)
- Groups rendered as labeled clusters of dots — position determined by:
  - **Angle**: stable per-group (hash of group name → angle)
  - **Distance from center**: recency of most recent session in group (closer = more recent)
- Individual sessions as contact blips within their group's cluster
- **Brightness**: active sessions glow bright (`#00E5FF`), idle ones fade toward dim (`#4A4E69`)
- Rotating sweep arm (animated, TachyonFX or manual tick) — purely aesthetic

### Interaction

- Cursor mode: navigate between groups/sessions on the radar with arrow keys
- Selecting a blip highlights the corresponding node in the session tree (bidirectional sync)
- Pressing `Enter` on a radar blip selects it (populates detail panel, same as tree selection)
- Tab switches focus between tree and radar

### Sync Protocol

- Expose a simple trait/interface: `set_highlight(session_id)` / `get_highlight() -> session_id`
- Both tree and radar read/write to shared selection state

## Acceptance Criteria

- [ ] Radar renders with range rings, crosshair, and labeled group clusters
- [ ] Session blips are positioned by group angle + recency distance
- [ ] Brightness reflects active/idle status
- [ ] Sweep arm animates smoothly
- [ ] Cursor navigation between blips works
- [ ] Selection syncs bidirectionally with tree (via shared state)
- [ ] Works with mock data

## Notes

This is the most visually ambitious widget. Use Ratatui's `Canvas` with `Marker::Braille` for smooth rendering. Develop against mock group/session data.
