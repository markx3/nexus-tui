use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border::Set as BorderSet;
use tachyonfx::{fx, Effect, Motion};

use crate::types::{PanelType, ThemeElement};

// ── Core palette ──────────────────────────────────────────────────────

pub const BG: Color = Color::Rgb(11, 12, 16);
pub const SURFACE: Color = Color::Rgb(20, 23, 38);

pub const TEXT: Color = Color::Rgb(200, 211, 245);
pub const DIM: Color = Color::Rgb(74, 78, 105);

pub const NEON_CYAN: Color = Color::Rgb(0, 229, 255);
pub const ACID_GREEN: Color = Color::Rgb(57, 255, 20);
pub const HAZARD: Color = Color::Rgb(247, 255, 74);

pub const BORDER: Color = Color::Rgb(40, 44, 72);
pub const NEON_MAGENTA: Color = Color::Rgb(255, 0, 128);

// ── Unicode decorators (used by integration layer) ──────────────────

pub const SEPARATOR: &str = "\u{2550}\u{2550}"; // ══

// ── Style lookup ──────────────────────────────────────────────────────

/// Centralized style lookup for any theme element.
pub fn style_for(element: ThemeElement) -> Style {
    match element {
        ThemeElement::Background => Style::new().bg(BG),
        ThemeElement::Surface => Style::new().bg(SURFACE),
        ThemeElement::Text => Style::new().fg(TEXT),
        ThemeElement::Dim => Style::new().fg(DIM),
        ThemeElement::NeonCyan => Style::new().fg(NEON_CYAN),
        ThemeElement::AcidGreen => Style::new().fg(ACID_GREEN).add_modifier(Modifier::BOLD),
        ThemeElement::Hazard => Style::new().fg(HAZARD),
        ThemeElement::NeonMagenta => Style::new().fg(NEON_MAGENTA),
        ThemeElement::Border => Style::new().fg(BORDER),
        ThemeElement::ActiveSession => Style::new().fg(ACID_GREEN),
        ThemeElement::IdleSession => Style::new().fg(DIM),
        ThemeElement::SelectedItem => Style::new()
            .bg(Color::Rgb(30, 35, 60))
            .fg(NEON_CYAN),
        ThemeElement::FocusedBorder => Style::new().fg(NEON_CYAN),
        ThemeElement::UnfocusedBorder => Style::new().fg(BORDER),
        ThemeElement::TreeIndent => Style::new().fg(BORDER),
        ThemeElement::RadarRing => Style::new().fg(BORDER),
        ThemeElement::RadarSweep => Style::new().fg(NEON_CYAN),
        ThemeElement::RadarBlip => Style::new().fg(ACID_GREEN),
        ThemeElement::TopBarLabel => Style::new().fg(DIM),
        ThemeElement::TopBarValue => Style::new().fg(TEXT).add_modifier(Modifier::BOLD),
        ThemeElement::DetailLabel => Style::new().fg(DIM),
        ThemeElement::DetailValue => Style::new().fg(TEXT),
        ThemeElement::ActivityGauge => Style::new().fg(ACID_GREEN),
    }
}

// ── Border sets ───────────────────────────────────────────────────────

/// Get the border character set for a panel type.
pub fn border_for(panel: PanelType) -> BorderSet<'static> {
    match panel {
        PanelType::TopBar => ratatui::symbols::border::DOUBLE,
        PanelType::SessionTree => ratatui::symbols::border::PLAIN,
        PanelType::Radar => ratatui::symbols::border::ROUNDED,
        PanelType::Detail => ratatui::symbols::border::PLAIN,
        PanelType::ActivityStrip => ratatui::symbols::border::DOUBLE,
    }
}

/// Get the border style for a panel, considering focus state.
pub fn border_style_for(panel: PanelType, is_focused: bool) -> Style {
    if is_focused {
        style_for(ThemeElement::FocusedBorder)
    } else {
        match panel {
            // Top bar and activity strip always get the focused accent
            PanelType::TopBar | PanelType::ActivityStrip => {
                style_for(ThemeElement::FocusedBorder)
            }
            _ => style_for(ThemeElement::UnfocusedBorder),
        }
    }
}

// ── TachyonFX effect presets ──────────────────────────────────────────

/// Staggered sweep-in boot animation for all five zones.
pub fn fx_boot() -> Vec<Effect> {
    vec![
        fx::sweep_in(Motion::LeftToRight, 15, 0, BG, 400u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, BG, 500u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, BG, 500u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, BG, 400u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, BG, 300u32),
    ]
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_for_returns_non_default_for_all_elements() {
        let elements = [
            ThemeElement::Background,
            ThemeElement::Surface,
            ThemeElement::Text,
            ThemeElement::Dim,
            ThemeElement::NeonCyan,
            ThemeElement::AcidGreen,
            ThemeElement::Hazard,
            ThemeElement::NeonMagenta,
            ThemeElement::Border,
            ThemeElement::ActiveSession,
            ThemeElement::IdleSession,
            ThemeElement::SelectedItem,
            ThemeElement::FocusedBorder,
            ThemeElement::UnfocusedBorder,
            ThemeElement::TreeIndent,
            ThemeElement::RadarRing,
            ThemeElement::RadarSweep,
            ThemeElement::RadarBlip,
            ThemeElement::TopBarLabel,
            ThemeElement::TopBarValue,
            ThemeElement::DetailLabel,
            ThemeElement::DetailValue,
            ThemeElement::ActivityGauge,
        ];

        let default_style = Style::default();
        for element in &elements {
            let style = style_for(*element);
            assert_ne!(
                style, default_style,
                "{element:?} returned a default style"
            );
        }
    }

    #[test]
    fn border_for_returns_valid_sets() {
        let panels = [
            PanelType::TopBar,
            PanelType::SessionTree,
            PanelType::Radar,
            PanelType::Detail,
            PanelType::ActivityStrip,
        ];

        for panel in &panels {
            let set = border_for(*panel);
            // Every border set must have non-empty corner characters
            assert!(
                !set.top_left.is_empty(),
                "{panel:?} border set has empty top_left"
            );
        }
    }

    #[test]
    fn border_style_focused_uses_neon_cyan() {
        let style = border_style_for(PanelType::Detail, true);
        assert_eq!(style, style_for(ThemeElement::FocusedBorder));
    }

    #[test]
    fn border_style_unfocused_uses_border_color() {
        let style = border_style_for(PanelType::Detail, false);
        assert_eq!(style, style_for(ThemeElement::UnfocusedBorder));
    }

    #[test]
    fn top_bar_always_focused_style() {
        let style = border_style_for(PanelType::TopBar, false);
        assert_eq!(style, style_for(ThemeElement::FocusedBorder));
    }

    #[test]
    fn fx_boot_returns_five_effects() {
        assert_eq!(fx_boot().len(), 5);
    }

}
