use ratatui::style::Color;
use tachyonfx::{fx, Effect, Motion};

pub const BG: Color = Color::Rgb(11, 12, 16);
pub const SURFACE: Color = Color::Rgb(20, 23, 38);

pub const TEXT: Color = Color::Rgb(200, 211, 245);
pub const DIM: Color = Color::Rgb(74, 78, 105);

pub const NEON_CYAN: Color = Color::Rgb(0, 229, 255);
pub const ACID_GREEN: Color = Color::Rgb(57, 255, 20);
pub const HAZARD: Color = Color::Rgb(247, 255, 74);

pub fn create_boot_effects() -> Vec<Effect> {
    vec![
        fx::sweep_in(Motion::LeftToRight, 15, 0, BG, 400u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, BG, 500u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, BG, 500u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, BG, 400u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, BG, 300u32),
    ]
}
