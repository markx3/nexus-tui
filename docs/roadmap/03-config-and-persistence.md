---
task: "03"
title: Config & Persistence Layer
type: feat
date: 2026-02-28
status: pending
depends_on: []
---

# 03 — Config & Persistence Layer

## Goal

Two data stores: a user-editable TOML config for group definitions and rules, and a SQLite database for app-managed session state.

## Scope

### TOML Config (`~/.config/nexus/config.toml` or similar)

- Group hierarchy definitions (nested groups)
- Auto-grouping rules: pattern-matching on `cwd` to assign sessions to groups
  - e.g., `[[rules]]` with `pattern = "Vault"` → `group = "Obsidian Vault"`
- Default working directory per group (used when creating new sessions)
- Optional theme overrides

### SQLite Database (`~/.local/share/nexus/nexus.db` or similar)

- Schema for:
  - `sessions` — cached metadata (id, slug, cwd, branch, last_active, message_count, tokens, model, topic_preview)
  - `groups` — hierarchy (id, name, parent_id)
  - `session_groups` — assignment mapping (session_id, group_id)
  - `session_overrides` — custom display names, manual notes
- CRUD operations for groups and session assignments
- Cache refresh: accept a `Vec<SessionInfo>` from the scanner and upsert

### Auto-Grouping Engine

- On scan, apply TOML rules to unassigned sessions
- Sessions without a matching rule go to an "Ungrouped" root node
- Manual assignments override auto-grouping

## Acceptance Criteria

- [ ] TOML config loads and validates (with sensible defaults if missing)
- [ ] SQLite schema creates cleanly on first run
- [ ] Groups support arbitrary nesting depth
- [ ] Auto-grouping rules correctly match sessions by cwd pattern
- [ ] Manual group assignments persist across restarts
- [ ] Unit tests for rule matching and CRUD operations

## Notes

Pure library module. Can be developed and tested without any UI.
