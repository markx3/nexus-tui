use rand::Rng;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::theme;
use crate::types::*;

// ---------------------------------------------------------------------------
// Game of Life engine
// ---------------------------------------------------------------------------

/// Animation loops every CYCLE_LEN frames, then re-seeds.
const CYCLE_LEN: usize = 128;

/// Initial live cells as (dx, dy) offsets from center (deterministic, for tests).
#[cfg(test)]
const SEED_OFFSETS: &[(isize, isize)] = &[
    // R-pentomino (chaotic, long-lived)
    (0, -1),
    (1, -1),
    (-1, 0),
    (0, 0),
    (0, 1),
    // Blinker top-left
    (-5, -2),
    (-5, -1),
    (-5, 0),
    // Block bottom-right (stable, acts as obstacle)
    (4, 1),
    (5, 1),
    (4, 2),
    (5, 2),
    // Glider seed bottom-left
    (-4, 2),
    (-3, 3),
    (-5, 3),
    (-4, 3),
    (-3, 2),
];

#[cfg(test)]
fn gol_seed(width: usize, height: usize) -> Vec<Vec<bool>> {
    let mut grid = vec![vec![false; width]; height];
    let cx = width as isize / 2;
    let cy = height as isize / 2;

    for &(dx, dy) in SEED_OFFSETS {
        let x = (cx + dx).rem_euclid(width as isize) as usize;
        let y = (cy + dy).rem_euclid(height as isize) as usize;
        grid[y][x] = true;
    }
    grid
}

fn random_seed(width: usize, height: usize) -> Vec<Vec<bool>> {
    let mut rng = rand::rng();
    (0..height)
        .map(|_| (0..width).map(|_| rng.random_bool(0.35)).collect())
        .collect()
}

fn count_neighbors(grid: &[Vec<bool>], x: usize, y: usize) -> u8 {
    let h = grid.len() as isize;
    let w = grid[0].len() as isize;
    let mut count = 0u8;
    for dy in [-1_isize, 0, 1] {
        for dx in [-1_isize, 0, 1] {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = (x as isize + dx).rem_euclid(w) as usize;
            let ny = (y as isize + dy).rem_euclid(h) as usize;
            if grid[ny][nx] {
                count += 1;
            }
        }
    }
    count
}

fn gol_step(grid: &[Vec<bool>]) -> Vec<Vec<bool>> {
    let h = grid.len();
    let w = grid[0].len();
    let mut next = vec![vec![false; w]; h];
    for y in 0..h {
        for x in 0..w {
            let n = count_neighbors(grid, x, y);
            next[y][x] = matches!((grid[y][x], n), (true, 2) | (true, 3) | (false, 3));
        }
    }
    next
}

/// Pick a display symbol based on cell position for visual variety.
fn symbol_for(x: usize, y: usize) -> char {
    match (x + y * 3) % 5 {
        0 => '◆',
        1 | 2 => '∙',
        _ => '·',
    }
}

// ---------------------------------------------------------------------------
// LogoState — cached Game of Life grid (one step per frame, not replay)
// ---------------------------------------------------------------------------

pub struct LogoState {
    grid: Vec<Vec<bool>>,
    prev_grid: Vec<Vec<bool>>,
    width: usize,
    height: usize,
    frame_count: usize,
}

impl LogoState {
    pub fn new() -> Self {
        Self {
            grid: Vec::new(),
            prev_grid: Vec::new(),
            width: 0,
            height: 0,
            frame_count: 0,
        }
    }

    /// Advance the GoL simulation by one step.
    ///
    /// Re-seeds on: size change, cycle limit, stagnation (grid == prev), or all dead.
    pub fn advance(&mut self, width: usize, height: usize) {
        if width != self.width || height != self.height || width < 5 || height < 3 {
            self.width = width;
            self.height = height;
            if width >= 5 && height >= 3 {
                self.grid = random_seed(width, height);
                self.prev_grid = Vec::new();
            } else {
                self.grid = Vec::new();
                self.prev_grid = Vec::new();
            }
            self.frame_count = 0;
            return;
        }

        // Forced reseed after CYCLE_LEN frames
        if self.frame_count >= CYCLE_LEN {
            self.grid = random_seed(width, height);
            self.prev_grid = Vec::new();
            self.frame_count = 0;
            return;
        }

        // Stagnation: grid unchanged from previous step (stable pattern)
        if self.grid == self.prev_grid {
            self.grid = random_seed(width, height);
            self.prev_grid = Vec::new();
            self.frame_count = 0;
            return;
        }

        // All dead
        let any_alive = self.grid.iter().any(|row| row.iter().any(|&c| c));
        if !any_alive {
            self.grid = random_seed(width, height);
            self.prev_grid = Vec::new();
            self.frame_count = 0;
            return;
        }

        self.prev_grid = self.grid.clone();
        self.grid = gol_step(&self.grid);
        self.frame_count += 1;
    }
}

// ---------------------------------------------------------------------------
// Frame generation (from cached state)
// ---------------------------------------------------------------------------

fn state_to_char_grid(state: &LogoState) -> Vec<Vec<char>> {
    let width = state.width;
    let height = state.height;
    let mut grid = vec![vec![' '; width]; height];

    if width < 5 || height < 3 {
        return grid;
    }

    // Convert live cells to display chars
    for (y, (grid_row, state_row)) in grid.iter_mut().zip(state.grid.iter()).enumerate() {
        for (x, (cell, &alive)) in grid_row.iter_mut().zip(state_row.iter()).enumerate() {
            if alive {
                *cell = symbol_for(x, y);
            }
        }
    }

    // Overlay static crosshair at center
    let cxi = width / 2;
    let cyi = height / 2;

    if cyi < height && cxi < width {
        grid[cyi][cxi] = '◉';
    }
    if cyi < height && cxi >= 2 && cxi + 2 < width {
        grid[cyi][cxi - 2] = '─';
        grid[cyi][cxi - 1] = '─';
        grid[cyi][cxi + 1] = '─';
        grid[cyi][cxi + 2] = '─';
    }
    if cxi < width && cyi >= 1 && cyi + 1 < height {
        grid[cyi - 1][cxi] = '┼';
        grid[cyi + 1][cxi] = '┼';
    }
    if cxi >= 1 && cxi + 1 < width && cyi >= 1 && cyi + 1 < height {
        grid[cyi - 1][cxi - 1] = '─';
        grid[cyi - 1][cxi + 1] = '─';
        grid[cyi + 1][cxi - 1] = '─';
        grid[cyi + 1][cxi + 1] = '─';
    }

    grid
}

/// Generate a char grid from seed (used only in tests for deterministic comparison).
#[cfg(test)]
fn generate_frame(width: usize, height: usize, frame_index: usize) -> Vec<Vec<char>> {
    let mut grid = vec![vec![' '; width]; height];

    if width < 5 || height < 3 {
        return grid;
    }

    let steps = frame_index % CYCLE_LEN;
    let mut state = gol_seed(width, height);
    for _ in 0..steps {
        state = gol_step(&state);
    }

    for y in 0..height {
        for x in 0..width {
            if state[y][x] {
                grid[y][x] = symbol_for(x, y);
            }
        }
    }

    let cxi = width / 2;
    let cyi = height / 2;

    if cyi < height && cxi < width {
        grid[cyi][cxi] = '◉';
    }
    if cyi < height && cxi >= 2 && cxi + 2 < width {
        grid[cyi][cxi - 2] = '─';
        grid[cyi][cxi - 1] = '─';
        grid[cyi][cxi + 1] = '─';
        grid[cyi][cxi + 2] = '─';
    }
    if cxi < width && cyi >= 1 && cyi + 1 < height {
        grid[cyi - 1][cxi] = '┼';
        grid[cyi + 1][cxi] = '┼';
    }
    if cxi >= 1 && cxi + 1 < width && cyi >= 1 && cyi + 1 < height {
        grid[cyi - 1][cxi - 1] = '─';
        grid[cyi - 1][cxi + 1] = '─';
        grid[cyi + 1][cxi - 1] = '─';
        grid[cyi + 1][cxi + 1] = '─';
    }

    grid
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

/// Render the animated logo panel.
pub fn render_logo(frame: &mut Frame, area: Rect, logo_state: &LogoState) {
    let block = Block::default()
        .title(Span::styled(
            " ◉ NEXUS ",
            theme::style_for(ThemeElement::LogoNexus),
        ))
        .borders(Borders::ALL)
        .border_set(theme::border_for(PanelType::Logo))
        .border_style(theme::border_style_for(PanelType::Logo, false))
        .style(theme::style_for(ThemeElement::Surface));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let grid = state_to_char_grid(logo_state);

    let agent_style = theme::style_for(ThemeElement::LogoAgent);
    let nexus_style = theme::style_for(ThemeElement::LogoNexus);

    let lines: Vec<Line> = grid
        .iter()
        .map(|row| {
            let spans: Vec<Span> = row
                .iter()
                .map(|&ch| match ch {
                    '◉' => Span::styled(ch.to_string(), nexus_style),
                    '∙' | '◆' | '·' | '─' | '│' | '┼' => {
                        Span::styled(ch.to_string(), agent_style)
                    }
                    _ => Span::styled(ch.to_string(), Style::default()),
                })
                .collect();
            Line::from(spans)
        })
        .collect();

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, inner);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_logo_no_panic() {
        let backend = TestBackend::new(20, 9);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = LogoState::new();
        state.advance(18, 7); // inner area after borders
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_logo(frame, area, &state);
            })
            .unwrap();
    }

    #[test]
    fn render_logo_zero_area() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = LogoState::new();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 0, 0);
                render_logo(frame, area, &state);
            })
            .unwrap();
    }

    #[test]
    fn render_logo_all_frames() {
        let backend = TestBackend::new(20, 9);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = LogoState::new();
        for _ in 0..24 {
            state.advance(18, 7);
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    render_logo(frame, area, &state);
                })
                .unwrap();
        }
    }

    #[test]
    fn render_logo_many_advances() {
        let mut state = LogoState::new();
        for _ in 0..1000 {
            state.advance(20, 9);
        }
        let backend = TestBackend::new(20, 9);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_logo(frame, area, &state);
            })
            .unwrap();
    }

    #[test]
    fn render_logo_small_area() {
        let backend = TestBackend::new(10, 4);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = LogoState::new();
        state.advance(8, 2);
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_logo(frame, area, &state);
            })
            .unwrap();
    }

    #[test]
    fn generate_frame_places_nexus_at_center() {
        let grid = generate_frame(16, 7, 0);
        let cx = 8;
        let cy = 3;
        assert_eq!(grid[cy][cx], '◉');
    }

    #[test]
    fn generate_frame_has_crosshair() {
        let grid = generate_frame(16, 7, 0);
        let cx = 8;
        let cy = 3;
        assert_eq!(grid[cy][cx - 1], '─');
        assert_eq!(grid[cy][cx + 1], '─');
        assert_eq!(grid[cy - 1][cx], '┼');
        assert_eq!(grid[cy + 1][cx], '┼');
    }

    #[test]
    fn generate_frame_tiny_grid() {
        let grid = generate_frame(3, 2, 0);
        assert_eq!(grid.len(), 2);
        assert_eq!(grid[0].len(), 3);
    }

    #[test]
    fn generate_frame_evolves() {
        let grid0 = generate_frame(20, 9, 0);
        let grid5 = generate_frame(20, 9, 5);
        assert_ne!(grid0, grid5);
    }

    #[test]
    fn gol_step_blinker_oscillates() {
        // Blinker: horizontal → vertical → horizontal
        let mut grid = vec![vec![false; 5]; 5];
        grid[2][1] = true;
        grid[2][2] = true;
        grid[2][3] = true;
        let next = gol_step(&grid);
        // Should become vertical
        assert!(!next[2][1]);
        assert!(next[1][2]);
        assert!(next[2][2]);
        assert!(next[3][2]);
        assert!(!next[2][3]);
        // Step again → back to horizontal
        let next2 = gol_step(&next);
        assert_eq!(grid, next2);
    }

    #[test]
    fn gol_step_block_is_stable() {
        let mut grid = vec![vec![false; 6]; 6];
        grid[2][2] = true;
        grid[2][3] = true;
        grid[3][2] = true;
        grid[3][3] = true;
        let next = gol_step(&grid);
        assert_eq!(grid, next);
    }

    #[test]
    fn gol_seed_has_live_cells() {
        let state = gol_seed(20, 9);
        let live: usize = state.iter().flat_map(|r| r.iter()).filter(|&&c| c).count();
        assert!(live > 0, "Seed should have live cells");
        assert_eq!(live, SEED_OFFSETS.len());
    }

    #[test]
    fn cycle_resets_to_seed() {
        let grid_0 = generate_frame(20, 9, 0);
        let grid_cycle = generate_frame(20, 9, CYCLE_LEN);
        assert_eq!(grid_0, grid_cycle, "Frame should loop after CYCLE_LEN");
    }

    #[test]
    fn render_logo_large_area() {
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = LogoState::new();
        state.advance(38, 18);
        state.advance(38, 18);
        state.advance(38, 18);
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_logo(frame, area, &state);
            })
            .unwrap();
    }

    #[test]
    fn logo_state_reseeds_on_size_change() {
        let mut state = LogoState::new();
        state.advance(20, 9);
        let grid1 = state.grid.clone();
        // Change size triggers reseed
        state.advance(30, 12);
        assert_eq!(state.width, 30);
        assert_eq!(state.height, 12);
        assert_ne!(grid1.len(), state.grid.len());
    }

    #[test]
    fn logo_state_reseeds_after_cycle_len() {
        let mut state = LogoState::new();
        // First advance seeds, then CYCLE_LEN more advances should trigger reseed
        for _ in 0..=CYCLE_LEN {
            state.advance(20, 9);
        }
        let grid_before = state.grid.clone();
        // Next advance hits frame_count >= CYCLE_LEN (or stagnation), must reseed
        state.advance(20, 9);
        // After reseed, frame_count resets to 0
        assert_eq!(state.frame_count, 0);
        // Grid should differ (random reseed vs settled state — vanishingly unlikely to match)
        assert_ne!(
            grid_before, state.grid,
            "Grid should change after forced reseed"
        );
    }
}
