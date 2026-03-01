---
status: pending
priority: p3
issue_id: "027"
tags: [code-review, simplicity, cleanup]
dependencies: []
---

# Dead Code Cleanup — ~75 LOC of Unused Code

## Problem Statement

Multiple modules contain dead code suppressed with `#[allow(dead_code)]` or simply never called:

1. **theme.rs:25-34** — Dead icon constants (`ICON_GROUP`, `ICON_SUBGROUP`, `ICON_ACTIVE`, `ICON_TARGET`) duplicating tree.rs icons
2. **theme.rs:110-113** — `create_boot_effects()` trivially wraps `fx_boot()`
3. **theme.rs:116-136** — Unused effects: `fx_transition()`, `fx_border_pulse()`, `fx_glitch_alert()`
4. **config.rs:29-30,90-92,151-156** — `tick_rate_ms` field parsed/validated but never read by app.rs
5. **config.rs:61,98-100** — `auto_launch` field and `default_true()` never read
6. **app.rs:21** — `dirty` flag set but never checked (comment says "Always redraw")
7. **mock.rs:116-123** — `mock_selection()` never called
8. **main.rs:28** — Redundant `db.init_schema()` (already called in `Database::open()`)

## Findings

- **Code Simplicity:** Findings #4-10, #13-14 — estimated ~75 LOC removable
- **Pattern Recognition:** Finding 2.8 — dead code accumulation
- **Architecture Strategist:** Findings 9.1, 11.3

## Proposed Solutions

### Option A: Remove all dead code (Recommended)
Delete all items listed above. For theme alias, keep `create_boot_effects` and inline `fx_boot`. For config, remove unused fields entirely.

- **Pros:** ~75 LOC reduction, no `#[allow(dead_code)]` suppression, cleaner modules
- **Cons:** Must re-add if features are implemented later
- **Effort:** Small
- **Risk:** Low

## Technical Details

**Affected files:** `src/theme.rs`, `src/config.rs`, `src/app.rs`, `src/mock.rs`, `src/main.rs`

## Acceptance Criteria

- [ ] No `#[allow(dead_code)]` annotations remain (except `config` field in App if kept intentionally)
- [ ] All tests pass
- [ ] `cargo build` produces no dead_code warnings

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Regular dead code pruning prevents accumulation |
