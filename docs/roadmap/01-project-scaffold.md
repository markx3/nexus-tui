---
task: "01"
title: Project Scaffold
type: feat
date: 2026-02-28
status: pending
depends_on: []
---

# 01 — Project Scaffold

## Goal

Bootstrap the Rust project with all dependencies, a working Ratatui event loop, and an empty Tactical Deck frame. This is the skeleton everything else builds on.

## Scope

- `cargo init` with appropriate package metadata
- `Cargo.toml` with all anticipated dependencies:
  - `ratatui`, `crossterm` (TUI framework + backend)
  - `tachyonfx` (animations)
  - `rusqlite` (SQLite)
  - `toml` / `serde` (config parsing)
  - `clap` (CLI args)
  - `tokio` or equivalent (async runtime if needed)
  - `dirs` (home directory resolution)
- Basic `main.rs`: terminal setup, event loop, graceful shutdown
- Empty placeholder zones matching the Tactical Deck layout (top bar, left panel, top-right, bottom-right, bottom strip)
- `.gitignore` for Rust projects
- Single-instance lock file check on startup

## Acceptance Criteria

- [ ] `cargo run` renders an empty Tactical Deck frame with 5 zones
- [ ] Pressing `q` exits cleanly (terminal restored)
- [ ] Second instance detects the first and exits with a message
- [ ] All dependencies compile without errors

## Notes

This task must complete before others can integrate, but others can develop independently against trait interfaces defined here.
