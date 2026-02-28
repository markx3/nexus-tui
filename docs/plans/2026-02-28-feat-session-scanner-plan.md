---
title: "Session Scanner"
type: feat
date: 2026-02-28
roadmap_task: "02"
---

# feat: Session Scanner

## Overview

Pure library module (`src/scanner.rs`) that discovers and parses Claude Code's JSONL session files into structured Rust types. No UI, no database — just file I/O and parsing. Returns `Vec<SessionInfo>` for downstream consumers (session tree, radar, detail panel).

Exposes a two-tier API: `scan_quick()` reads only the first N lines per file for fast metadata extraction (~0.1ms/file), while `scan_full()` parses entire files for accurate token counts and message totals (~90ms for a 20MB file).

## Problem Statement

Claude Code stores session data as JSONL files under `~/.claude/projects/`. Each project gets an encoded directory name, each session is a `<uuid>.jsonl` file, and sub-agents live in `<uuid>/subagents/`. There is no structured index — to know what sessions exist, you must walk the filesystem and parse raw JSONL. Nexus needs this data to populate its session tree, radar, and detail panels.

## Proposed Solution

### Data Types

```rust
// src/scanner.rs

pub struct SessionInfo {
    pub session_id: String,            // UUID from filename / sessionId field
    pub slug: Option<String>,          // Human name from first assistant entry (e.g., "joyful-hopping-lake")
    pub cwd: Option<PathBuf>,          // Working directory from JSONL cwd field (canonical project path)
    pub project_dir: PathBuf,          // Raw encoded directory name (e.g., "-Users-foo-Code")
    pub git_branch: Option<String>,    // Last observed gitBranch value
    pub model: Option<String>,         // First non-<synthetic> model from assistant entries
    pub version: Option<String>,       // Claude Code version string
    pub first_message: Option<String>, // First real user message (topic preview, truncated)
    pub message_count: u32,            // user (non-meta) + assistant (non-synthetic) entries
    pub token_usage: TokenUsage,       // Actual counts from message.usage fields
    pub subagent_count: u16,           // Count of *.jsonl files in <uuid>/subagents/
    pub last_active: String,           // ISO 8601 timestamp of last entry (or file mtime fallback)
    pub source_file: PathBuf,          // Absolute path to the .jsonl file (needed for --resume)
}

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl Default for TokenUsage {
    fn default() -> Self {
        Self { input_tokens: 0, output_tokens: 0 }
    }
}
```

**Field semantics for multi-valued fields:**
- `git_branch`: Last observed value (most current)
- `model`: First non-`<synthetic>` value (the "real" model used)
- `message_count`: Count of `user` entries where `isMeta != true` + `assistant` entries where model != `<synthetic>`
- `first_message`: First `user` entry that is non-meta AND whose content is a plain string (skip `<command-message>` tags, skip array-of-tool-result content). Truncated to 200 chars. `None` if nothing suitable found.

### Two-Tier API

```rust
/// Quick scan: reads first ~50 lines per file.
/// Extracts: session_id, slug, cwd, git_branch, model, version, first_message, subagent_count.
/// Fields that require full parse: message_count = 0, token_usage = default, last_active = file mtime.
pub fn scan_quick(projects_dir: &Path) -> Result<Vec<SessionInfo>>

/// Full scan: reads every line of every file.
/// All fields are fully populated.
pub fn scan_full(projects_dir: &Path) -> Result<Vec<SessionInfo>>
```

Both functions accept a `projects_dir` argument (typically `~/.claude/projects/`) rather than hardcoding the path, making them testable with fixture directories.

### JSONL Entry Dispatch

Each line is parsed as `serde_json::Value` and dispatched on the `type` field:

| `type` | Fields extracted |
|--------|-----------------|
| `"user"` | `sessionId`, `cwd`, `gitBranch`, `version`, `timestamp`; if `isMeta != true`: first message content, message count increment |
| `"assistant"` | `sessionId`, `slug`, `message.model`, `message.usage.{input,output}_tokens`, `timestamp`; message count increment (if model != `<synthetic>`) |
| `"progress"` | `sessionId`, `cwd`, `gitBranch`, `version`, `timestamp` |
| `"queue-operation"` | `timestamp` only |
| `"file-history-snapshot"` | Skip (no useful top-level metadata) |
| `"system"` | `timestamp` only |
| `"pr-link"` | Skip for now (future: extract PR URL) |
| Unknown | Skip silently |

### Scanning Algorithm

```
scan(projects_dir):
  if !projects_dir.exists() → return Ok(vec![])

  for each subdirectory in projects_dir:
    project_dir = subdirectory name (encoded path)

    for each *.jsonl file in subdirectory (not recursive — skip subagent dirs):
      session_id = filename stem (UUID)

      subagent_count = count *.jsonl files in <uuid>/subagents/ (if dir exists)

      parse JSONL file line by line:
        for each line:
          try parse as JSON → on error: skip line (warn on first error per file)
          dispatch on "type" field
          extract fields into SessionInfo builder

      if session has no cwd AND no sessionId → skip (snapshot-only session)

      push SessionInfo to results

  return results
```

### Error Handling

Follow the project's `color_eyre::Result` pattern with graceful degradation:

- **`~/.claude/projects/` doesn't exist**: Return `Ok(vec![])` with info-level log
- **Permission denied on a directory**: Skip it, log warning, continue scanning
- **Malformed JSON line**: Skip the line, log warning (only first occurrence per file to avoid log spam)
- **Empty JSONL file**: Skip, debug-level log
- **Snapshot-only session** (no cwd, no sessionId): Skip entirely, debug-level log
- **Incomplete last line** (file actively being written): The last line may be truncated JSON — handle parse failure on last line gracefully (not an error)
- **File I/O error mid-read**: Wrap in context with the filename, skip that session

Use `eprintln!` for warnings (no logging framework yet — keep it simple, add `tracing` later if needed).

### Dependencies to Add

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

No additional dependencies needed:
- `dirs` (already present) — for `dirs::home_dir()` to find `~/.claude/`
- `std::fs::read_dir` — sufficient for 2-level directory walk (no `walkdir` needed)
- Timestamps stored as strings — no `chrono` needed yet
- Session IDs stored as strings — no `uuid` crate needed

## Technical Considerations

**Performance**: The two-tier API is the main performance strategy. Quick scan reads ~50 lines (covers all metadata fields for most sessions). Full scan is O(total JSONL bytes) but only needed for accurate token counts. On the real corpus (205 sessions, largest 20MB), quick scan should complete in <100ms.

**Concurrency**: Not needed for v1. Single-threaded sequential scan. Can add rayon later if the corpus grows beyond ~1000 sessions and quick scan becomes slow.

**Memory**: Parse one line at a time with `BufReader`. Never load a full file into memory. `serde_json::Value` per line is acceptable (each line is typically <10KB, with rare spikes for tool results).

**Testing**: Use `#[cfg(test)] mod tests` at the bottom of `scanner.rs`. Create `tests/fixtures/` with synthetic JSONL files covering the edge cases below. Do NOT use real session files as fixtures (they contain private data).

## Acceptance Criteria

- [x] `scan_quick()` returns `Vec<SessionInfo>` from `~/.claude/projects/`
- [x] `scan_full()` returns `Vec<SessionInfo>` with populated `message_count` and `token_usage`
- [x] `cwd` field is read from JSONL entries (not decoded from directory name)
- [x] `slug` is extracted from the first entry that has one (assistant or progress)
- [x] `first_message` skips `isMeta: true` entries and `<command-message>` content
- [x] `subagent_count` accurately counts `*.jsonl` files in `<uuid>/subagents/`
- [x] Malformed/empty JSONL files are skipped with a warning (no panic, no error propagation)
- [x] Snapshot-only sessions (no cwd, no sessionId) are skipped
- [x] Missing `~/.claude/projects/` returns empty vec (not an error)
- [x] Unit tests with synthetic JSONL fixtures covering:
  - Normal session (user + assistant exchanges)
  - Session with no slug (no assistant response)
  - Session with `isMeta` user messages
  - Snapshot-only session (should be skipped)
  - Empty file
  - Malformed JSON line mid-file
  - Session with subagents

## Success Metrics

- Quick scan of the real `~/.claude/projects/` completes in <500ms
- Zero panics on any real-world session file
- All test fixtures pass

## Dependencies & Risks

**Dependencies**: None — this is an independent library module. Task 01 (scaffold) is complete.

**Risks**:
- Claude Code may change the JSONL format in future versions. Mitigated by parsing with `serde_json::Value` (not rigid struct deserialization) and skipping unknown fields/types.
- Large corpora (>1000 sessions) may make full scan slow. Mitigated by the two-tier API — downstream consumers use quick scan for initial load, full scan on demand.

## References

- Roadmap spec: `docs/roadmap/02-session-scanner.md`
- Brainstorm: `docs/brainstorms/2026-02-28-claude-session-manager-brainstorm.md`
- Real JSONL format: `~/.claude/projects/-Users-marcosfelipeeipper-Code-nexus/*.jsonl`
- JSONL entry types: `user`, `assistant`, `progress`, `queue-operation`, `file-history-snapshot`, `system`, `pr-link`
- Existing patterns: `src/app.rs` (color_eyre, struct visibility), `src/main.rs` (module declarations)
