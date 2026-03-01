---
status: complete
priority: p2
issue_id: "037"
tags: [code-review, architecture, agent-parity]
dependencies: ["021"]
---
# Add CLI Subcommands for New TUI Capabilities (Agent Parity)

## Problem Statement
The plan introduces 8 new interactive capabilities (send-keys, capture, delete, rename, move, new-group, conversation-log, resize) that are exclusively accessible through the TUI. No CLI subcommands expose them. This drops agent-parity from 73% (8/11) to 42% (8/19). An agent that wants to send input to a session or read its output has no programmatic path.

## Findings
- Existing CLI: 6 subcommands (list, show, new, launch, kill, groups) in src/cli.rs
- Plan adds 8 TUI-only capabilities with zero CLI equivalents
- Internal primitives (TmuxManager methods, DB operations) are already clean and ready for CLI exposure
- Existing --json flag pattern and serde derives support machine-readable output
- Related: todo #021 documents pre-existing CLI agent parity debt
- Found by: Agent-Native Reviewer

## Proposed Solutions

### Option 1: Add core CLI subcommands in Phase 3 (Recommended)
**Approach:** Add `nexus send <session> <text>`, `nexus capture <session>`, `nexus delete <session>`, `nexus rename <session> <name>`, `nexus move <session> --group <group>`, `nexus group create <name>`. These reuse existing TmuxManager and DB methods — purely CLI wiring (~80-100 LOC in cli.rs + main.rs).
**Pros:** Agents can interact with sessions programmatically, follows existing CLI patterns, small effort
**Cons:** Adds ~100 LOC to Phase 3 scope
**Effort:** 2-3 hours
**Risk:** Low

### Option 2: Defer CLI parity to follow-up plan
**Approach:** Ship TUI-only, add CLI later.
**Pros:** Reduces Phase 3 scope
**Cons:** Agent parity drops to 42%, agents cannot use the core new feature
**Effort:** 0 now, same effort later
**Risk:** Low (but agent users blocked)

## Technical Details
**Affected files:** src/cli.rs, src/main.rs
**Priority subcommands:** `send` and `capture` (core new capability), `delete`/`rename`/`move`/`group` (CRUD parity)

## Acceptance Criteria
- [ ] `nexus send <session> <text>` forwards text to tmux session
- [ ] `nexus capture <session>` outputs current pane content
- [ ] `nexus delete`, `rename`, `move`, `group` subcommands work
- [ ] All new subcommands support --json flag

## Work Log
### 2026-02-28 - Code Review Discovery
**By:** Agent-Native Reviewer
### 2026-03-01 - Incorporated into Plan Revision
**Actions:** Added CLI Parity task block to Phase 3 with send, capture, delete, rename, move, group create subcommands. Added --json flag requirement. Added acceptance criterion for CLI parity.
