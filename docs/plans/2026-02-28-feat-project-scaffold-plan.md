---
title: "feat: Project Scaffold"
type: feat
date: 2026-02-28
roadmap_task: "01"
---

# Project Scaffold

## Overview

Bootstrap the Nexus Rust project: Cargo setup, dependencies, event loop, Tactical Deck layout with empty labeled zones, single-instance lock, TachyonFX boot animation, and graceful shutdown. After this, `cargo run` renders a cyberpunk frame and exits cleanly on `q`.

## File Structure

```
nexus/
├── Cargo.toml
├── .gitignore
├── src/
│   ├── main.rs         # Entry point: lock, CLI, init, run, restore
│   ├── app.rs          # App struct, event loop, tick, shutdown flag
│   ├── ui.rs           # Tactical Deck layout + zone rendering
│   ├── theme.rs        # Color constants + border sets (minimal)
│   └── cli.rs          # Clap derive args
```

## Implementation Steps

### Step 1: Cargo.toml

```toml
[package]
name = "nexus"
version = "0.1.0"
edition = "2021"
description = "Cyberpunk TUI session manager for Claude Code"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
tachyonfx = "0.9"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
clap = { version = "4.5", features = ["derive"] }
dirs = "5.0"
color-eyre = "0.6"
fslock = "0.2"

[profile.release]
opt-level = 3
lto = true
strip = true
```

**Version alignment note:** tachyonfx must be compatible with the ratatui version. Verify actual compatibility on crates.io before pinning — if tachyonfx requires a different ratatui version, align to it. The exact versions above may need adjustment; resolve during `cargo build`.

### Step 2: .gitignore

Standard Rust gitignore: `/target`, `Cargo.lock` (binary crate — actually include it), `.DS_Store`.

### Step 3: src/cli.rs — CLI Arguments

```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nexus", version, about = "Cyberpunk TUI session manager for Claude Code")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,
}
```

Minimal. More flags added in later tasks.

### Step 4: src/theme.rs — Color Palette & Borders

Define the cyberpunk palette as constants. Only what's needed for the scaffold (zone borders, background, text).

```rust
use ratatui::style::Color;

// Background
pub const BG: Color = Color::Rgb(11, 12, 16);         // #0B0C10
pub const SURFACE: Color = Color::Rgb(20, 23, 38);    // #141726

// Text
pub const TEXT: Color = Color::Rgb(200, 211, 245);     // #C8D3F5
pub const DIM: Color = Color::Rgb(74, 78, 105);        // #4A4E69

// Neon
pub const NEON_CYAN: Color = Color::Rgb(0, 229, 255);  // #00E5FF
pub const NEON_MAGENTA: Color = Color::Rgb(255, 0, 255); // #FF00FF
pub const ACID_GREEN: Color = Color::Rgb(57, 255, 20);  // #39FF14
pub const HAZARD: Color = Color::Rgb(247, 255, 74);     // #F7FF4A
```

Also define border style helpers (heavy for structural, dashed for holographic). Expand as needed in task 08.

### Step 5: src/app.rs — App Struct & Event Loop

```rust
pub struct App {
    pub should_quit: bool,
    last_tick: Instant,
    // TachyonFX boot effects (one per zone)
    boot_effects: Vec<Effect>,
}
```

**Event loop** — synchronous, poll-based, 16ms tick:

1. `terminal.draw(|frame| ui::draw(frame, &mut app))`
2. `crossterm::event::poll(timeout)` — blocks for remaining tick time
3. Handle `Event::Key` → `q` sets `should_quit = true`
4. Handle `Event::Resize` → no special action (layout recomputes automatically)
5. If tick elapsed → `app.on_tick()` (currently no-op, used for animation timing)

**Tick rate:** 16ms (~60fps). TachyonFX effects need elapsed time passed each frame.

**Graceful shutdown:** The loop exits when `should_quit` is true. `ratatui::restore()` in `main.rs` handles terminal cleanup. `color-eyre` + `ratatui::init()` install a panic hook that restores the terminal on crash.

### Step 6: src/ui.rs — Tactical Deck Layout

Layout splits using Ratatui's `Layout` with the `.areas()` destructuring pattern:

```rust
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Check minimum size
    if area.width < 80 || area.height < 24 {
        // Render a "terminal too small" message instead
        let msg = Paragraph::new("Terminal too small. Minimum: 80x24")
            .style(Style::new().fg(theme::HAZARD));
        frame.render_widget(msg, area);
        return;
    }

    // Top-level: top bar, main area, bottom strip
    let [top_bar, main_area, bottom_strip] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
    ]).areas(area);

    // Main area: left panel, right column
    let [left_panel, right_column] = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ]).areas(main_area);

    // Right column: radar (top), detail (bottom)
    let [radar_area, detail_area] = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ]).areas(right_column);

    // Render each zone as a labeled bordered block
    draw_top_bar(frame, top_bar);
    draw_session_tree(frame, left_panel);
    draw_radar(frame, radar_area);
    draw_detail(frame, detail_area);
    draw_activity_strip(frame, bottom_strip);

    // Apply TachyonFX boot effects (if still running)
    // ...
}
```

**Zone placeholders** — each zone gets a `Block` with cyberpunk borders and a labeled `Paragraph`:

| Zone | Title | Placeholder Text | Border Style |
|------|-------|-----------------|--------------|
| Top bar | `SYS:ONLINE ══ SESSIONS:-- ══ ACTIVE:-- ══ <date>` | Status line with dim placeholders | Heavy (`━━━`) |
| Left panel | `SESSION TREE` | `"No sessions loaded"` | Heavy |
| Radar | `SESSION RADAR` | `"◉"` centered (radar center marker) | Dashed (`╌╌╌`) |
| Detail | `DETAIL` | `"Select a session to view details"` | Dashed (`╌╌╌`) |
| Activity strip | `ACTIVITY` | `"No active sessions"` | Heavy |

Zone titles rendered with `NEON_CYAN`. Placeholder text in `DIM`. Background `SURFACE`. Structural borders in `NEON_CYAN`, holographic borders in `DIM`.

### Step 7: src/main.rs — Entry Point

```rust
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let _cli = cli::Cli::parse();
    let _lock = acquire_lock()?;

    let terminal = ratatui::init();
    let result = app::App::new().run(terminal);
    ratatui::restore();
    result
}
```

**Single-instance lock:**

```rust
fn acquire_lock() -> color_eyre::Result<fslock::LockFile> {
    let lock_dir = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("nexus");
    std::fs::create_dir_all(&lock_dir)?;

    let mut lock = fslock::LockFile::open(&lock_dir.join("nexus.lock"))?;

    if !lock.try_lock()? {
        eprintln!("nexus: another instance is already running");
        eprintln!("  If this is a stale lock, remove: {}", lock_dir.join("nexus.lock").display());
        std::process::exit(1);
    }

    Ok(lock)
}
```

`fslock` uses OS-level advisory locking (`flock(2)` on Unix). The lock is automatically released when the process exits — even on crash or `kill -9`. No stale lock problem.

### Step 8: TachyonFX Boot Animation

On startup, create a `sweep_in` effect for each zone (staggered left-to-right). Effects are applied in `ui::draw` after rendering widgets:

```rust
// In App::new()
let boot_effects = vec![
    fx::sweep_in(Direction::LeftToRight, 15, 0, Color::Rgb(11, 12, 16), 400),    // top bar
    fx::sweep_in(Direction::LeftToRight, 15, 100, Color::Rgb(11, 12, 16), 500),  // left panel
    fx::sweep_in(Direction::LeftToRight, 15, 200, Color::Rgb(11, 12, 16), 500),  // radar
    fx::sweep_in(Direction::LeftToRight, 15, 300, Color::Rgb(11, 12, 16), 400),  // detail
    fx::sweep_in(Direction::LeftToRight, 15, 400, Color::Rgb(11, 12, 16), 300),  // activity strip
];
```

Each zone's effect is rendered over its area. Effects self-expire — after ~800ms total, all zones are fully visible and effects stop.

**Note:** TachyonFX `render_effect` requires `use tachyonfx::EffectRenderer` in scope to extend `Frame`.

## Edge Case Decisions

| Edge Case | Decision | Rationale |
|-----------|----------|-----------|
| Terminal too small | Show "Terminal too small. Minimum: 80x24" | Simple, avoids panic on degenerate layout |
| No true color support | Don't detect. Assume modern terminal. | Target audience uses iTerm2/Alacritty/Kitty. YAGNI. |
| SIGINT (Ctrl+C) | Crossterm captures as key event in raw mode. Handle as quit. | No `signal-hook` needed. |
| Panic during render | `ratatui::init()` panic hook restores terminal. `color-eyre` formats nicely. | Built-in, no extra work. |
| Resize during boot animation | Layout recomputes. Animation continues with new elapsed time. | Ratatui handles layout automatically. Visually fine. |
| Config file missing | Ignore for scaffold (config loading is task 03). `--config` flag is parsed but unused. | YAGNI. |
| Lock dir doesn't exist | `create_dir_all` creates it. Falls back to temp dir. | Robust, no user intervention. |

## Acceptance Criteria

- [ ] `cargo build` succeeds with all dependencies
- [ ] `cargo run` renders the Tactical Deck with 5 labeled zones
- [ ] Cyberpunk color palette visible (neon cyan borders on dark background)
- [ ] Boot animation plays (zones sweep in left-to-right with stagger)
- [ ] `q` exits cleanly (terminal fully restored)
- [ ] `Ctrl+C` exits cleanly
- [ ] Second instance prints "another instance is already running" and exits
- [ ] Terminal too small (<80x24) shows a warning message instead of the layout
- [ ] Panic during render restores terminal (test by adding a deliberate `panic!()`)

## References

- [Ratatui 0.30 Highlights](https://ratatui.rs/highlights/v030/) — Layout `.areas()`, `ratatui::init()`
- [Ratatui Component Template](https://ratatui.rs/templates/component/project-structure/) — File structure pattern
- [TachyonFX GitHub](https://github.com/ratatui/tachyonfx) — Effect integration pattern
- [fslock docs](https://docs.rs/fslock/latest/fslock/) — Advisory file locking
- Brainstorm: `docs/brainstorms/2026-02-28-claude-session-manager-brainstorm.md`
- Roadmap task: `docs/roadmap/01-project-scaffold.md`
