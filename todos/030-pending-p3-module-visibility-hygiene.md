---
status: pending
priority: p3
issue_id: "030"
tags: [code-review, hygiene, build]
dependencies: []
---

# Module Visibility and Build Hygiene

## Problem Statement

Two build hygiene issues:

1. **`pub mod` on binary crate:** Most modules are `pub mod` but this is a binary crate with no library surface. All modules could be `mod` or `pub(crate)`.
   - Location: `src/main.rs:1-12`

2. **`mock.rs` in release binary:** `pub mod mock` is compiled into the release binary. It should be gated behind `#[cfg(test)]`.
   - Location: `src/main.rs:6`

## Findings

- **Architecture Strategist:** Finding 11.2 — "mock module is compiled and included in the release binary."
- **Pattern Recognition:** Finding 3.3 — "all modules could be `mod` instead of `pub mod`."

## Proposed Solutions

### Option A: Fix both (Recommended)
1. Change `pub mod` to `mod` for all modules in main.rs
2. Gate mock.rs: `#[cfg(test)] mod mock;`

- **Effort:** Small
- **Risk:** Low (may need `pub(crate)` if cross-module test access is needed)

## Technical Details

**Affected files:** `src/main.rs`

## Acceptance Criteria

- [ ] No `pub mod` in binary crate (use `mod` or `pub(crate)`)
- [ ] `mock.rs` only compiled in test builds
- [ ] `cargo build --release` succeeds
- [ ] All tests pass

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-28 | Created from full-codebase review | Binary crates don't need pub modules |
