---
status: pending
priority: p2
issue_id: "003"
tags: [code-review, security]
dependencies: []
---

# Scanner Follows Symlinks — Could Read Outside Intended Directory

## Problem Statement

`Path::is_dir()` and `Path::is_file()` follow symlinks (they call `std::fs::metadata`, not `symlink_metadata`). A symlink inside `~/.claude/projects/` pointing to an arbitrary directory would be followed, and `.jsonl` files in that target would be read and parsed.

## Findings

- **Security Sentinel:** Rated as Medium severity. Provided proof of concept:
  ```bash
  mkdir ~/.claude/projects/evil-project
  ln -s /etc/passwd evil-project/steal-me.jsonl
  ```
- Mitigated by the fact that there is no network exfiltration channel (TUI-only), and an attacker with write access to `~/.claude/` already has significant access.

**Affected locations:**
- `src/scanner.rs:236` — `project_path.is_dir()`
- `src/scanner.rs:257` — `file_path.is_file()`
- `src/scanner.rs:352` — `subagents_dir.is_dir()`

## Proposed Solutions

### Option A: Reject symlinks with `symlink_metadata` (Recommended)
```rust
let meta = std::fs::symlink_metadata(&project_path);
if meta.map(|m| m.file_type().is_symlink()).unwrap_or(false) {
    continue;
}
```
- **Pros:** Defense-in-depth, ~10 lines of code
- **Cons:** Could reject legitimate symlinked project directories (unlikely use case)
- **Effort:** Small
- **Risk:** Low

### Option B: Document as accepted risk
- **Pros:** No code change
- **Cons:** Leaves the vulnerability open
- **Effort:** Trivial
- **Risk:** Low (given threat model)

## Acceptance Criteria

- [ ] Symlinks in project directories are skipped
- [ ] Symlinks in JSONL file entries are skipped
- [ ] Symlinks in subagent directories are skipped

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from code review of task 02 | |
