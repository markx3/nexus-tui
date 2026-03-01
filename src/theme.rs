use std::sync::atomic::{AtomicUsize, Ordering};

use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border::Set as BorderSet;
use tachyonfx::{fx, Effect, Motion};

use crate::types::{PanelType, ThemeElement};

// ── Palette definition ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub surface: Color,
    pub text: Color,
    pub dim: Color,
    pub primary: Color,
    pub secondary: Color,
    pub hazard: Color,
    pub border: Color,
    pub accent: Color,
}

pub const PALETTE_COUNT: usize = 8;
pub const DEFAULT_PALETTE_INDEX: usize = 6; // Retrowave Pure

pub const PALETTE_NAMES: [&str; PALETTE_COUNT] = [
    "Current Baseline",
    "Outrun Sunset",
    "Cyberpunk 2077",
    "Blade Runner 2049",
    "Neon Deep Ocean",
    "Synthwave Nights",
    "Retrowave Pure",
    "Matrix Phosphor",
];

const PALETTES: [Palette; PALETTE_COUNT] = [
    // 0: Current Baseline
    Palette {
        bg:        Color::Rgb(11, 12, 16),
        surface:   Color::Rgb(20, 23, 38),
        text:      Color::Rgb(200, 211, 245),
        dim:       Color::Rgb(74, 78, 105),
        primary:   Color::Rgb(0, 229, 255),
        secondary: Color::Rgb(57, 255, 20),
        hazard:    Color::Rgb(247, 255, 74),
        border:    Color::Rgb(40, 44, 72),
        accent:    Color::Rgb(255, 0, 128),
    },
    // 1: Outrun Sunset
    Palette {
        bg:        Color::Rgb(13, 2, 33),
        surface:   Color::Rgb(21, 5, 53),
        text:      Color::Rgb(224, 208, 255),
        dim:       Color::Rgb(92, 77, 125),
        primary:   Color::Rgb(235, 100, 185),
        secondary: Color::Rgb(0, 240, 208),
        hazard:    Color::Rgb(255, 200, 87),
        border:    Color::Rgb(42, 22, 84),
        accent:    Color::Rgb(255, 108, 17),
    },
    // 2: Cyberpunk 2077
    Palette {
        bg:        Color::Rgb(10, 10, 15),
        surface:   Color::Rgb(20, 20, 30),
        text:      Color::Rgb(232, 232, 204),
        dim:       Color::Rgb(90, 90, 74),
        primary:   Color::Rgb(243, 230, 0),
        secondary: Color::Rgb(0, 240, 255),
        hazard:    Color::Rgb(255, 0, 60),
        border:    Color::Rgb(42, 42, 56),
        accent:    Color::Rgb(189, 0, 255),
    },
    // 3: Blade Runner 2049
    Palette {
        bg:        Color::Rgb(12, 10, 8),
        surface:   Color::Rgb(26, 22, 16),
        text:      Color::Rgb(212, 197, 169),
        dim:       Color::Rgb(107, 93, 69),
        primary:   Color::Rgb(247, 139, 4),
        secondary: Color::Rgb(79, 193, 233),
        hazard:    Color::Rgb(255, 61, 61),
        border:    Color::Rgb(51, 41, 29),
        accent:    Color::Rgb(201, 93, 30),
    },
    // 4: Neon Deep Ocean
    Palette {
        bg:        Color::Rgb(3, 11, 16),
        surface:   Color::Rgb(8, 24, 32),
        text:      Color::Rgb(184, 216, 224),
        dim:       Color::Rgb(60, 90, 102),
        primary:   Color::Rgb(10, 189, 198),
        secondary: Color::Rgb(234, 0, 217),
        hazard:    Color::Rgb(255, 230, 109),
        border:    Color::Rgb(21, 48, 64),
        accent:    Color::Rgb(113, 243, 65),
    },
    // 5: Synthwave Nights
    Palette {
        bg:        Color::Rgb(11, 14, 20),
        surface:   Color::Rgb(19, 24, 34),
        text:      Color::Rgb(172, 189, 211),
        dim:       Color::Rgb(73, 90, 115),
        primary:   Color::Rgb(4, 172, 238),
        secondary: Color::Rgb(247, 110, 153),
        hazard:    Color::Rgb(255, 209, 102),
        border:    Color::Rgb(31, 42, 62),
        accent:    Color::Rgb(54, 245, 199),
    },
    // 6: Retrowave Pure (default)
    Palette {
        bg:        Color::Rgb(14, 5, 32),
        surface:   Color::Rgb(23, 10, 48),
        text:      Color::Rgb(216, 200, 248),
        dim:       Color::Rgb(94, 76, 128),
        primary:   Color::Rgb(131, 56, 236),
        secondary: Color::Rgb(255, 0, 110),
        hazard:    Color::Rgb(255, 190, 11),
        border:    Color::Rgb(45, 24, 80),
        accent:    Color::Rgb(58, 134, 255),
    },
    // 7: Matrix Phosphor
    Palette {
        bg:        Color::Rgb(5, 8, 5),
        surface:   Color::Rgb(10, 18, 10),
        text:      Color::Rgb(168, 213, 160),
        dim:       Color::Rgb(58, 92, 58),
        primary:   Color::Rgb(0, 255, 65),
        secondary: Color::Rgb(0, 187, 48),
        hazard:    Color::Rgb(255, 215, 0),
        border:    Color::Rgb(26, 46, 26),
        accent:    Color::Rgb(64, 224, 208),
    },
];

// ── AtomicUsize palette index ───────────────────────────────────────

static THEME_INDEX: AtomicUsize = AtomicUsize::new(DEFAULT_PALETTE_INDEX);

pub fn active_palette() -> &'static Palette {
    &PALETTES[THEME_INDEX.load(Ordering::Relaxed)]
}

pub fn current_index() -> usize {
    THEME_INDEX.load(Ordering::Relaxed)
}

pub fn current_name() -> &'static str {
    PALETTE_NAMES[current_index()]
}

pub fn set_theme(index: usize) {
    THEME_INDEX.store(index % PALETTE_COUNT, Ordering::Relaxed);
}

pub fn next_theme() -> usize {
    let new = (current_index() + 1) % PALETTE_COUNT;
    set_theme(new);
    new
}

pub fn prev_theme() -> usize {
    let cur = current_index();
    let new = if cur == 0 { PALETTE_COUNT - 1 } else { cur - 1 };
    set_theme(new);
    new
}

// ── Color accessor functions ────────────────────────────────────────

pub fn bg() -> Color { active_palette().bg }
pub fn surface() -> Color { active_palette().surface }
pub fn text() -> Color { active_palette().text }
pub fn dim() -> Color { active_palette().dim }
pub fn primary() -> Color { active_palette().primary }
pub fn secondary() -> Color { active_palette().secondary }
pub fn hazard() -> Color { active_palette().hazard }
pub fn border() -> Color { active_palette().border }
pub fn accent() -> Color { active_palette().accent }

/// Derive a selection highlight background by brightening `surface`.
pub fn derive_selection_bg() -> Color {
    match active_palette().surface {
        Color::Rgb(r, g, b) => Color::Rgb(
            r.saturating_add(10),
            g.saturating_add(12),
            b.saturating_add(22),
        ),
        other => other,
    }
}

/// Derive an unfocused selection background (slightly dimmer than focused).
pub fn derive_unfocused_selection_bg() -> Color {
    match active_palette().surface {
        Color::Rgb(r, g, b) => Color::Rgb(
            r.saturating_add(5),
            g.saturating_add(6),
            b.saturating_add(11),
        ),
        other => other,
    }
}

// ── Unicode decorators (used by integration layer) ──────────────────

pub const SEPARATOR: &str = "\u{2550}\u{2550}"; // ══

// ── Style lookup ────────────────────────────────────────────────────

/// Centralized style lookup for any theme element.
pub fn style_for(element: ThemeElement) -> Style {
    match element {
        ThemeElement::Background => Style::new().bg(bg()),
        ThemeElement::Surface => Style::new().bg(surface()),
        ThemeElement::Text => Style::new().fg(text()),
        ThemeElement::Dim => Style::new().fg(dim()),
        ThemeElement::NeonCyan => Style::new().fg(primary()),
        ThemeElement::AcidGreen => Style::new().fg(secondary()).add_modifier(Modifier::BOLD),
        ThemeElement::Hazard => Style::new().fg(hazard()),
        ThemeElement::NeonMagenta => Style::new().fg(accent()),
        ThemeElement::Border => Style::new().fg(border()),
        ThemeElement::ActiveSession => Style::new().fg(secondary()),
        ThemeElement::IdleSession => Style::new().fg(dim()),
        ThemeElement::SelectedItem => Style::new()
            .bg(derive_selection_bg())
            .fg(primary()),
        ThemeElement::FocusedBorder => Style::new().fg(primary()),
        ThemeElement::UnfocusedBorder => Style::new().fg(border()),
        ThemeElement::TreeIndent => Style::new().fg(border()),
        ThemeElement::TopBarLabel => Style::new().fg(dim()),
        ThemeElement::TopBarValue => Style::new().fg(text()).add_modifier(Modifier::BOLD),
        ThemeElement::InteractorTitle => Style::new().fg(primary()).add_modifier(Modifier::BOLD),
        ThemeElement::ConversationHuman => Style::new().fg(primary()),
        ThemeElement::ConversationAssistant => Style::new().fg(secondary()),
        ThemeElement::LogoAgent => Style::new().fg(dim()),
        ThemeElement::LogoNexus => Style::new().fg(primary()),
    }
}

// ── Border sets ─────────────────────────────────────────────────────

/// Get the border character set for a panel type.
pub fn border_for(panel: PanelType) -> BorderSet<'static> {
    match panel {
        PanelType::TopBar => ratatui::symbols::border::DOUBLE,
        PanelType::SessionTree => ratatui::symbols::border::PLAIN,
        PanelType::SessionInteractor => ratatui::symbols::border::PLAIN,
        PanelType::Logo => ratatui::symbols::border::PLAIN,
    }
}

/// Get the border style for a panel, considering focus state.
pub fn border_style_for(panel: PanelType, is_focused: bool) -> Style {
    if is_focused {
        style_for(ThemeElement::FocusedBorder)
    } else {
        match panel {
            PanelType::TopBar => style_for(ThemeElement::FocusedBorder),
            _ => style_for(ThemeElement::UnfocusedBorder),
        }
    }
}

// ── TachyonFX effect presets ────────────────────────────────────────

/// Staggered sweep-in boot animation for three zones (top_bar, tree, right_column).
pub fn fx_boot() -> Vec<Effect> {
    let bg_color = bg();
    vec![
        fx::sweep_in(Motion::LeftToRight, 15, 0, bg_color, 400u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, bg_color, 500u32),
        fx::sweep_in(Motion::LeftToRight, 15, 0, bg_color, 500u32),
    ]
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Reset theme to default before each test to avoid cross-test interference
    fn reset_theme() {
        set_theme(DEFAULT_PALETTE_INDEX);
    }

    #[test]
    fn style_for_returns_non_default_for_all_elements() {
        reset_theme();
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
            ThemeElement::TopBarLabel,
            ThemeElement::TopBarValue,
            ThemeElement::InteractorTitle,
            ThemeElement::ConversationHuman,
            ThemeElement::ConversationAssistant,
            ThemeElement::LogoAgent,
            ThemeElement::LogoNexus,
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
            PanelType::SessionInteractor,
            PanelType::Logo,
        ];

        for panel in &panels {
            let set = border_for(*panel);
            assert!(
                !set.top_left.is_empty(),
                "{panel:?} border set has empty top_left"
            );
        }
    }

    #[test]
    fn border_style_focused_uses_primary() {
        reset_theme();
        let style = border_style_for(PanelType::SessionTree, true);
        assert_eq!(style, style_for(ThemeElement::FocusedBorder));
    }

    #[test]
    fn border_style_unfocused_uses_border_color() {
        reset_theme();
        let style = border_style_for(PanelType::SessionTree, false);
        assert_eq!(style, style_for(ThemeElement::UnfocusedBorder));
    }

    #[test]
    fn top_bar_always_focused_style() {
        reset_theme();
        let style = border_style_for(PanelType::TopBar, false);
        assert_eq!(style, style_for(ThemeElement::FocusedBorder));
    }

    #[test]
    fn fx_boot_returns_three_effects() {
        assert_eq!(fx_boot().len(), 3);
    }

    #[test]
    fn next_theme_cycles_through_all() {
        reset_theme();
        set_theme(0);
        for expected in 1..PALETTE_COUNT {
            assert_eq!(next_theme(), expected);
        }
        // Wraps back to 0
        assert_eq!(next_theme(), 0);
    }

    #[test]
    fn prev_theme_wraps_from_zero() {
        reset_theme();
        set_theme(0);
        assert_eq!(prev_theme(), PALETTE_COUNT - 1);
    }

    #[test]
    fn palette_names_count_matches() {
        assert_eq!(PALETTE_NAMES.len(), PALETTE_COUNT);
    }

    #[test]
    fn all_palettes_have_rgb_colors() {
        for (i, palette) in PALETTES.iter().enumerate() {
            let colors = [
                palette.bg, palette.surface, palette.text, palette.dim,
                palette.primary, palette.secondary, palette.hazard,
                palette.border, palette.accent,
            ];
            for color in &colors {
                assert!(
                    matches!(color, Color::Rgb(_, _, _)),
                    "Palette {} ({}) has a non-RGB color",
                    i,
                    PALETTE_NAMES[i]
                );
            }
        }
    }

    #[test]
    fn set_theme_clamps_to_valid_index() {
        set_theme(PALETTE_COUNT + 5);
        assert!(current_index() < PALETTE_COUNT);
        reset_theme();
    }

    #[test]
    fn current_name_returns_valid_string() {
        reset_theme();
        assert_eq!(current_name(), "Retrowave Pure");
    }

    #[test]
    fn derive_selection_bg_returns_rgb() {
        reset_theme();
        assert!(matches!(derive_selection_bg(), Color::Rgb(_, _, _)));
    }
}
