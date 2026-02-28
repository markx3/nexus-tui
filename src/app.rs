use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use tachyonfx::Effect;

use crate::ui;

const TICK_RATE: Duration = Duration::from_millis(16);

pub struct App {
    pub should_quit: bool,
    last_tick: Instant,
    pub boot_effects: Vec<Effect>,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            last_tick: Instant::now(),
            boot_effects: ui::create_boot_effects(),
        }
    }

    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            let elapsed = self.last_tick.elapsed();

            terminal.draw(|frame| ui::draw(frame, &mut self, elapsed))?;

            let timeout = TICK_RATE.saturating_sub(self.last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => self.should_quit = true,
                            KeyCode::Char('c') if key.modifiers.contains(
                                crossterm::event::KeyModifiers::CONTROL,
                            ) => self.should_quit = true,
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
