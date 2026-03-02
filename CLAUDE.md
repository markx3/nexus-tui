# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```sh
cargo build                    # dev build
cargo test                     # all tests (~160, runs in <1s)
cargo test <test_name>         # single test by name
cargo test <module>::tests     # run one module's tests (e.g. cargo test db::tests)
cargo clippy -- -D warnings    # lint (CI enforces zero warnings)
cargo fmt                      # format (CI runs cargo fmt --check with stable toolchain)
cargo install --path .         # install binary locally
```

CI runs `cargo fmt --check`, then `clippy -D warnings`, then `cargo test` on Ubuntu stable toolchain.

## Architecture

Nexus is a cyberpunk-themed TUI session manager for Claude Code, built with Rust/Ratatui/TachyonFX. It wraps Claude Code sessions in tmux panes and provides a dashboard with live terminal preview, session grouping, and full CRUD.

### Two entry paths

`main.rs` dispatches: no subcommand → TUI mode (`run_tui`), subcommand → CLI mode (`run_cli`). Both share the same `Database` and `TmuxManager`.

### Core modules

- **`app.rs`** — `App` struct owns all state (tree, tmux, db, input mode, capture worker). Runs the event loop at 16ms ticks, polls tmux every 2s, routes input via `InteractorState`.
- **`ui.rs`** — Layout engine. Three zones: top_bar (3 rows), tree (13% left), interactor (fills right). Renders modal overlays (text input, confirm, group picker, help).
- **`db.rs`** — SQLite via rusqlite (bundled), WAL mode, at `~/.local/share/nexus/nexus.db`. Schema migrations in `init_schema()`. Owns the tree query (`get_tree`/`get_visible_tree`).
- **`tmux.rs`** — `TmuxManager` wraps a dedicated socket (`tmux -L nexus`). Handles launch, kill, send-keys, capture-pane, resize. Validates targets to prevent injection.
- **`types.rs`** — Shared vocabulary: `TreeNode`, `SessionSummary`, `NexusCommand`, `RouteResult`, `InputMode`, `InputContext`, `ThemeElement`, `PanelType`.

### Widget system (`src/widgets/`)

Each widget is a stateless render function paired with a state module:
- **tree.rs / tree_state.rs** — Session tree. `TreeState` flattens `TreeNode` hierarchy into `FlatNode` vec with cursor tracking and expand/collapse cache.
- **interactor.rs / interactor_state.rs** — Right panel showing live terminal output or conversation logs. `InteractorState` owns the input routing pipeline, returns `RouteResult` enum to `App`.
- **top_bar.rs** — Status bar with session counts and theme name.
- **logo.rs** — Game of Life animation in the bottom-left corner.

### Key design patterns

- **Alt-only keybinding namespace**: All Nexus controls use Alt+key. Every other keystroke passes through to the active tmux session. This is the core UX invariant.
- **RouteResult enum**: `InteractorState` processes every event and returns `Handled` (consumed locally — scroll, tmux forward), `NexusCommand(cmd)` (App dispatches), or `Ignored`.
- **Capture worker**: Background thread polls `tmux capture-pane`, parses ANSI via `ansi-to-tui`, sends `Text<'static>` back to the main thread.
- **Theme system**: 8 palettes stored in `theme.rs`, runtime switching via `AtomicUsize`. All color access goes through `theme::style_for(ThemeElement)`.

### Data flow

```
User input → crossterm Event → InteractorState::route_event()
  → RouteResult::Handled (scroll/tmux forward)
  → RouteResult::NexusCommand → App dispatches (CRUD, modal, etc.)
```

### File paths at runtime

- Config: `~/.config/nexus/config.toml`
- Database: `~/.local/share/nexus/nexus.db`
- Lock: `~/.cache/nexus/nexus.lock`
- Tmux socket: `tmux -L nexus`
