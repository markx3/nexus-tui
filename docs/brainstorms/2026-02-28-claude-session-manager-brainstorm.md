---
date: 2026-02-28
topic: claude-session-manager
---

# Nexus — Claude Session Manager

## What We're Building

A cyberpunk-themed TUI for managing Claude Code sessions across all projects. The core experience is a **command center dashboard** that gives you instant spatial awareness of your entire session landscape — then lets you act on it (resume, create, organize, launch).

The TUI reads Claude's native session storage (`~/.claude/projects/`), organizes sessions into a **user-defined hierarchical group structure**, and launches/resumes sessions in **tmux panes** so the TUI stays running as your persistent control plane.

Built in **Rust with Ratatui**, styled with a neon-on-dark cyberpunk aesthetic, animated with TachyonFX.

## Why This Approach

**Problem:** Claude Code's native `--resume` picker is scoped to the current working directory. With 1,500+ sessions across 34 projects, there's no way to see the full landscape, group related sessions, or quickly switch context between workflows (e.g., Obsidian note-taking vs. a multi-agent research project).

**Existing tools** (agent-of-empires, agent-deck, ccrider, ccmanager) focus on tmux-based session spawning/monitoring but none offer hierarchical grouping, a spatial radar visualization, or a cyberpunk command-center experience.

**Chosen approach: "Tactical Deck"** — a four-zone command center layout with a collapsible session tree, an interactive radar visualization, a rich detail panel, and a live activity strip. This was chosen over simpler tree-only or tab-based layouts because the radar provides unique spatial awareness that no other tool offers, and the layout makes full use of modern widescreen terminals.

## Layout

```
┏━━ SYS:ONLINE ━━ SESSIONS:47 ━━ ACTIVE:3 ━━ MEM:4.1G ━━ 28-FEB-2026 ━━┓
┃                                    ┃                                     ┃
┃  SESSION TREE                      ┃  SESSION RADAR                      ┃
┃  (collapsible hierarchy)           ┃  (Braille canvas, interactive)      ┃
┃  Groups + sessions, j/k nav        ┃  Groups as clusters, recency =     ┃
┃  Expand/collapse with Enter        ┃  distance from center, brightness   ┃
┃  ▶ marks active sessions           ┃  = activity level                   ┃
┃                                    ┃                                     ┃
┃                                    ┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┃
┃                                    ┃  DETAIL PANEL                       ┃
┃                                    ┃  Session metadata, first message    ┃
┃                                    ┃  preview, sub-agent status,         ┃
┃                                    ┃  action keybindings                 ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┃
┃  ACTIVITY STRIP — live gauges for all running sessions                   ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

**Zones:**
1. **Top bar** (3 rows) — System status: session count, active count, system stats, timestamp
2. **Left panel** (~50% width) — Collapsible session tree with hierarchical groups
3. **Top-right** (~50% width, ~50% height) — Interactive Braille radar visualization
4. **Bottom-right** — Rich detail panel for selected session
5. **Bottom strip** (3 rows) — Live activity gauges for all running tmux sessions

**Radar-Tree sync:** Selecting a node in the tree highlights it on the radar. Navigating to a blip on the radar highlights it in the tree. Two views, one data model.

## Key Decisions

- **Rust / Ratatui**: Chosen over Go/Bubble Tea for TachyonFX animation power and Ratatui's Canvas widget (needed for the radar). The user wants to explore Rust.
- **Hierarchical groups**: Groups can contain sub-groups and sessions at any level. Tree structure, not flat tags.
- **Auto-group with manual override**: Sessions auto-group by working directory. Users can create custom groups, move sessions, rename things — all from within the TUI.
- **TOML config + SQLite**: TOML file for user-editable group definitions and rules. SQLite for session metadata cache and state (auto-managed by the app).
- **Tmux integration**: Resuming/creating a session opens Claude in a new tmux pane. The TUI stays running as the control plane.
- **Rich session metadata**: Name/slug, working directory, last active, first message preview, git branch, message count, model, active/idle status, token usage, sub-agent count.
- **Full CRUD from TUI**: Create groups, create sessions (inheriting group defaults like working directory), rename, move, delete — all without leaving the TUI.
- **Interactive radar**: Not just ambient — navigable with cursor, synced bidirectionally with the tree.

## Cyberpunk Aesthetic

**Color palette:**
- Background: `#0B0C10` (blue-black void, never pure black)
- Surface: `#141726` (elevated panels)
- Border/dim: `#1C2333` (subtle outlines)
- Text: `#C8D3F5` (cool blue-white)
- Primary neon: `#00E5FF` (electric cyan — active elements, primary borders)
- Secondary: `#FF00FF` (hot magenta — alerts, input focus)
- Active: `#39FF14` (acid green — running sessions)
- Hazard: `#F7FF4A` (acid yellow — warnings)
- Dim: `#4A4E69` (muted purple-gray — inactive)

**Animation (TachyonFX):**
- Boot sequence: panels sweep in left-to-right with staggered delays
- Panel transitions: `coalesce` (text materializes from noise) when selecting a new session
- Radar sweep: rotating arm with fading contact trails
- Active border pulse: slow HSL shift on focused panel borders
- Error/alert: brief glitch effect (HSL shift to red, dissolve + coalesce)

**Visual elements:**
- Dashed borders (`╌╌╌`) for "projected/holographic" panels
- Heavy borders (`━━━`) for structural frames
- Unicode decorators: `◉` (radar center), `◈` (group icon), `⬡` (sub-group), `▶` (active session)
- Block-character progress gauges: `████▒░░░`
- Braille-rendered radar with concentric range rings

## Data Model

**From Claude's native storage (read-only):**
- Session files: `~/.claude/projects//<encoded-path>/<uuid>.jsonl`
- Per session: `sessionId`, `slug`, `cwd`, `gitBranch`, `timestamp`, `version`, first user message content
- Sub-agents: `<uuid>/subagents/*.jsonl`

**App-managed (SQLite):**
- Session metadata cache (parsed from JSONL, refreshed on demand)
- Group assignments (which session belongs to which group)
- Session display names (custom overrides of the slug)
- Last-seen timestamps, message counts, token estimates

**User-editable (TOML config):**
- Group hierarchy definitions
- Auto-grouping rules (e.g., `cwd contains "Vault" -> "Obsidian Vault"`)
- Default working directories per group
- Theme overrides (optional)

## Resolved Questions

- **Grouping model**: Hierarchical (not tags, not flat). Sessions at any level.
- **Tech stack**: Rust / Ratatui (not Go/Bubble Tea, not extending agentboard).
- **Session launch**: Tmux pane (not replace TUI, not suspend/resume).
- **Persistence split**: TOML for human-editable config, SQLite for app state.
- **Radar interactivity**: Fully interactive, bidirectionally synced with tree.

## Open Questions

None — all resolved.

## Resolved Questions (continued)

- **Project name**: `nexus` — central hub connecting all sessions.
- **Tmux dependency**: Hard requirement. Nexus provides its own shortcuts (e.g., `Ctrl+Q` to detach from a session and return to Nexus) so users don't need raw tmux knowledge.
- **Session scanning**: Simple scan on startup, cache in SQLite. No premature optimization — reading first lines of 1,500 files is trivial on modern hardware.
- **Multi-terminal**: Single instance only. Nexus detects if another instance is running and refuses to start (or attaches to the existing tmux session).

## Next Steps

-> `/workflows:plan` for implementation details
