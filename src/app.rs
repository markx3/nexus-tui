use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use tachyonfx::Effect;

use crate::{theme, ui};

const TICK_RATE: Duration = Duration::from_millis(16);

pub struct App {
    pub should_quit: bool,
    pub(crate) dirty: bool,
    pub(crate) boot_done: bool,
    last_tick: Instant,
    pub(crate) boot_effects: Vec<Effect>,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            dirty: true,
            boot_done: false,
            last_tick: Instant::now(),
            boot_effects: theme::create_boot_effects(),
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_tick);

            if self.dirty || !self.boot_done {
                terminal.draw(|frame| ui::draw(frame, &mut self, elapsed))?;
                self.dirty = false;
            }

            let poll_timeout = if self.boot_done {
                Duration::from_millis(250)
            } else {
                TICK_RATE.saturating_sub(now.elapsed())
            };

            if event::poll(poll_timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.dirty = true;
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                self.should_quit = true;
                            }
                            KeyCode::Char('c')
                                if key.modifiers
                                    .contains(crossterm::event::KeyModifiers::CONTROL) =>
                            {
                                self.should_quit = true;
                            }
                            _ => {}
                        }
                    }
                }
            }

            if self.last_tick.elapsed() >= TICK_RATE {
                self.last_tick = Instant::now();
            }
        }

        Ok(())
    }
}
