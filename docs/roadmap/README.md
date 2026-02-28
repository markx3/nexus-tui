---
date: 2026-02-28
---

# Nexus Roadmap

## Task Dependency Graph

```
01 Project Scaffold ─┐
                     │
02 Session Scanner ──┤
                     │
03 Config & Persist ─┤
                     │
04 Tmux Manager ─────┼──→ 09 Layout & Wiring (integration)
                     │
05 Session Tree ─────┤
                     │
06 Radar Widget ─────┤
                     │
07 Info Panels ──────┤
                     │
08 Cyberpunk Theme ──┘
```

## Tasks

| # | Task | Type | Independent | Status |
|---|------|------|-------------|--------|
| 01 | [Project Scaffold](01-project-scaffold.md) | Foundation | Start here | Done |
| 02 | [Session Scanner](02-session-scanner.md) | Library | Yes | Pending |
| 03 | [Config & Persistence](03-config-and-persistence.md) | Library | Yes | Pending |
| 04 | [Tmux Manager](04-tmux-manager.md) | Library | Yes | Pending |
| 05 | [Session Tree Widget](05-session-tree-widget.md) | UI | Yes (mock data) | Pending |
| 06 | [Radar Widget](06-radar-widget.md) | UI | Yes (mock data) | Pending |
| 07 | [Info Panels](07-info-panels.md) | UI | Yes (mock data) | Pending |
| 08 | [Cyberpunk Theme](08-cyberpunk-theme.md) | Styling | Yes | Pending |
| 09 | [Layout & Wiring](09-layout-and-wiring.md) | Integration | Depends on all | Pending |

## Parallelism

After **01** is done, tasks **02–08** can all be worked on in parallel. Each is self-contained:

- **02, 03, 04** are pure library modules — no UI, testable independently
- **05, 06, 07** are UI widgets — develop against mock data
- **08** is a styling module — demonstrable in a test harness

**09** is the final integration pass that wires everything together.

## Per-Task Planning

Each task gets its own detailed implementation plan (via `/workflows:plan`) before work begins.
