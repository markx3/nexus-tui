---
status: pending
priority: p2
issue_id: "025"
tags: [code-review, security]
dependencies: []
---

# /tmp Fallback When HOME Is Unset — Insecure Default

## Problem Statement

When `dirs::home_dir()` returns `None`, config paths fall back to `/tmp`:
- `config.rs:77-81` — `default_projects_dir()` → `/tmp/.claude/projects`
- `config.rs:83-88` — `default_db_path()` → `/tmp/nexus/nexus.db`

On shared systems, `/tmp` is world-writable. This would allow other users to plant malicious JSONL files or read/modify the SQLite database.

## Findings

- **Security Sentinel:** Finding #13 (LOW) — "unlikely edge case but insecure failure mode."

## Proposed Solutions

### Option A: Fail with clear error (Recommended)
Replace `unwrap_or_else(|| PathBuf::from("/tmp"))` with an explicit error:
```rust
dirs::home_dir().ok_or_else(|| eyre!("Cannot determine home directory. Set $HOME."))?
```

- **Pros:** Fail-safe, no insecure default, clear error message
- **Cons:** App won't start without HOME (which is the correct behavior)
- **Effort:** Small
- **Risk:** Low

## Technical Details

**Affected files:** `src/config.rs`

## Acceptance Criteria

- [ ] No `/tmp` fallback in any path construction
- [ ] Clear error message when HOME is unset
- [ ] App refuses to start rather than using world-writable paths

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Fail-safe > insecure default |
