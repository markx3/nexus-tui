---
status: pending
priority: p2
issue_id: "021"
tags: [code-review, architecture, agent-native]
dependencies: []
---

# Add CLI Subcommands — Zero Agent Accessibility

## Problem Statement

Nexus has an empty `Cli` struct with no subcommands. Every operation (list, launch, kill, scan, group) is locked behind the interactive TUI. An agent cannot use Nexus programmatically. Running `nexus` always enters the event loop.

**0 of 10 user-facing capabilities are agent-accessible.**

## Findings

- **Agent-Native Reviewer:** Score 0/10 — "NEEDS WORK." Identified missing CLI surface for all operations. Internal primitives are well-factored and ready for exposure.

**Key gaps:**
- No `nexus list` — can't list sessions without TUI
- No `nexus launch <id>` — can't launch without navigating tree
- No `nexus kill <id>` — kill_window exists but is unreachable
- No `--json` flag — no machine-readable output
- No `--config` or `--db-path` overrides
- Instance lock prevents CLI use alongside running TUI

## Proposed Solutions

### Option A: Add clap subcommands (Recommended)
```
nexus                             # Launch TUI (default)
nexus list [--json]               # List all sessions
nexus show <session-id> [--json]  # Show session detail
nexus launch <session-id>         # Launch/resume tmux session
nexus kill <session-id>           # Kill tmux session
nexus scan [--full] [--json]      # Force rescan
nexus groups [--json]             # List groups
```

Add `#[derive(Serialize)]` to output types. Only acquire exclusive lock for TUI mode.

- **Pros:** Full agent parity, scriptable, CI-friendly
- **Cons:** Moderate effort (~200 LOC), needs Serialize derives
- **Effort:** Large
- **Risk:** Low

### Option B: Minimal `list` + `launch` only
Add just the two most critical subcommands.

- **Pros:** Small effort, covers the 80% use case
- **Cons:** Still missing kill, scan, groups
- **Effort:** Medium
- **Risk:** Low

## Recommended Action

Option A — the internal primitives already exist, this is surface wiring.

## Technical Details

**Affected files:** `src/cli.rs`, `src/main.rs`, `src/types.rs` (add Serialize)
**Components:** CLI layer, types layer

## Acceptance Criteria

- [ ] `nexus list --json` outputs sessions as JSON array
- [ ] `nexus launch <id>` launches/resumes without TUI
- [ ] `nexus kill <id>` kills a tmux session
- [ ] Read-only commands work while TUI is running (no lock conflict)
- [ ] All output types derive `Serialize`

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Agent-native score: 0/10 |
