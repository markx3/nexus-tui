---
task: "02"
title: Session Scanner
type: feat
date: 2026-02-28
status: pending
depends_on: []
---

# 02 — Session Scanner

## Goal

Library module that discovers and parses Claude Code's JSONL session files into structured Rust types. Pure data extraction — no UI, no database.

## Scope

- Discover session files by walking `~/.claude/projects//<encoded-path>/*.jsonl`
- Decode project directory names back to real paths (e.g., `-Users-foo-Code` → `/Users/foo/Code`)
- Parse each session's JSONL to extract:
  - `session_id` (UUID)
  - `slug` (human-readable name like `joyful-hopping-lake`)
  - `cwd` (working directory)
  - `git_branch`
  - `timestamp` (last message)
  - `model` / `version`
  - First user message content (topic preview)
  - Message count
  - Token usage estimate (count messages × rough estimate, or from stats if available)
- Discover sub-agents: `<uuid>/subagents/*.jsonl` — count and basic status
- Return a `Vec<SessionInfo>` struct

## Acceptance Criteria

- [ ] Scans the real `~/.claude/projects/` directory
- [ ] Correctly parses session metadata from JSONL (tested against real files)
- [ ] Handles malformed/empty JSONL files gracefully (skip, log warning)
- [ ] Sub-agent count is accurate
- [ ] Unit tests with sample JSONL fixtures

## Notes

This is a pure library module (`src/scanner.rs` or `src/scanner/mod.rs`). No UI dependencies. Can be developed and tested independently.
