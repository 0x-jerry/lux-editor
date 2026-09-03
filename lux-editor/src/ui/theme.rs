//! App-level themes: chrome `Visuals` plus a coupled syntax theme.
//!
//! A theme answers two questions at once: how the chrome looks and which
//! syntect theme colors the code, so token/background contrast is guaranteed.
//! `Auto` follows the OS preference reported by [`egui::Context::system_theme`].

use crate::config::EditorSettings;
use eframe::egui;
use egui::{Color32, CornerRadius, Stroke, Visuals};
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

/// User-facing theme choice, persisted in config as a string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeChoice {
    Auto,
    Dark,
    Light,
}

impl ThemeChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }

    pub fn value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn from_value(value: &str) -> Self {
        match value {
            "dark" => Self::Dark,
            "light" => Self::Light,
            _ => Self::Auto,
        }
    }
}

/// Collapse `Auto` against the system theme; never returns [`ThemeChoice::Auto`].
pub fn resolve(choice: ThemeChoice, system: Option<egui::Theme>) -> ThemeChoice {
    match choice {
        ThemeChoice::Dark => ThemeChoice::Dark,
        ThemeChoice::Light => ThemeChoice::Light,
        ThemeChoice::Auto => match system {
            Some(egui::Theme::Light) => ThemeChoice::Light,
            _ => ThemeChoice::Dark,
        },
    }
}

/// The coupled syntax theme for a choice, resolving `Auto` against the system.
pub fn syntax_theme_for(choice: ThemeChoice, system: Option<egui::Theme>) -> &'static str {
    match choice {
        ThemeChoice::Dark => "base16-ocean.dark",
        ThemeChoice::Light => "base16-ocean.light",
        ThemeChoice::Auto => match system {
            Some(egui::Theme::Light) => "base16-ocean.light",
            _ => "base16-ocean.dark",
        },
    }
}

/// A resolved theme: chrome `Visuals` ready for `ctx.set_visuals`.
pub struct AppTheme {
    pub visuals: Visuals,
}

impl AppTheme {
    /// Visuals for a resolved choice (`Auto` falls back to dark).
    pub fn resolve(theme: ThemeChoice) -> Self {
        match theme {
            ThemeChoice::Light => light(),
            ThemeChoice::Dark | ThemeChoice::Auto => dark(),
        }
    }
}

fn dark() -> AppTheme {
    let accent = Color32::from_rgb(0x6a, 0xa1, 0xff);
    let code_bg = Color32::from_rgb(0x2b, 0x30, 0x3b); // base16-ocean.dark background
    let text = Color32::from_rgb(0xc0, 0xc5, 0xce);
    let weak = Color32::from_rgb(0x8a, 0x95, 0xa6);
    let border = Color32::from_rgb(0x3a, 0x40, 0x4d);
    let raised = Color32::from_rgb(0x25, 0x28, 0x30);
    let hover = Color32::from_rgb(0x33, 0x39, 0x45);
    let active = Color32::from_rgb(0x41, 0x4a, 0x5a);
    let mut visuals = Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(0x1e, 0x20, 0x27);
    visuals.window_fill = Color32::from_rgb(0x1a, 0x1c, 0x22);
    visuals.extreme_bg_color = Color32::from_rgb(0x17, 0x19, 0x1e);
    visuals.faint_bg_color = Color32::from_rgb(0x2c, 0x30, 0x3a);
    visuals.code_bg_color = code_bg;
    visuals.window_stroke = Stroke::new(1.0, border);
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0x6a, 0xa1, 0xff, 64);
    visuals.selection.stroke = Stroke::new(1.0, accent);
    visuals.hyperlink_color = accent;
    visuals.weak_text_color = Some(weak);
    let palette = Palette {
        text,
        weak,
        border,
        raised,
        hover,
        active,
        accent,
    };
    style_widgets(&mut visuals, &palette);
    AppTheme { visuals }
}

fn light() -> AppTheme {
    let accent = Color32::from_rgb(0x0b, 0x6b, 0xcb);
    let code_bg = Color32::from_rgb(0xef, 0xf1, 0xf5); // base16-ocean.light background
    let text = Color32::from_rgb(0x4f, 0x5b, 0x66);
    let weak = Color32::from_rgb(0x8a, 0x95, 0xa3);
    let border = Color32::from_rgb(0xd3, 0xd9, 0xe2);
    let raised = Color32::from_rgb(0xe4, 0xe8, 0xed);
    let hover = Color32::from_rgb(0xd8, 0xdf, 0xe8);
    let active = Color32::from_rgb(0xc8, 0xd3, 0xe0);
    let mut visuals = Visuals::light();
    visuals.panel_fill = Color32::from_rgb(0xf0, 0xf2, 0xf5);
    visuals.window_fill = Color32::from_rgb(0xfb, 0xfc, 0xfd);
    visuals.extreme_bg_color = Color32::from_rgb(0xe7, 0xea, 0xef);
    visuals.faint_bg_color = Color32::from_rgb(0xe2, 0xe6, 0xec);
    visuals.code_bg_color = code_bg;
    visuals.window_stroke = Stroke::new(1.0, border);
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0x0b, 0x6b, 0xcb, 40);
    visuals.selection.stroke = Stroke::new(1.0, accent);
    visuals.hyperlink_color = accent;
    visuals.weak_text_color = Some(weak);
    let palette = Palette {
        text,
        weak,
        border,
        raised,
        hover,
        active,
        accent,
    };
    style_widgets(&mut visuals, &palette);
    AppTheme { visuals }
}

#[derive(Clone, Copy)]
struct Palette {
    text: Color32,
    weak: Color32,
    border: Color32,
    raised: Color32,
    hover: Color32,
    active: Color32,
    accent: Color32,
}

fn style_widgets(visuals: &mut Visuals, palette: &Palette) {
    let Palette {
        text,
        weak,
        border,
        raised,
        hover,
        active,
        accent,
    } = *palette;
    let corner = CornerRadius::same(4);
    visuals.widgets.noninteractive = egui::style::WidgetVisuals {
        bg_fill: raised,
        weak_bg_fill: Color32::TRANSPARENT,
        bg_stroke: Stroke::new(1.0, border),
        corner_radius: CornerRadius::same(2),
        fg_stroke: Stroke::new(1.0, text),
        expansion: 0.0,
    };
    visuals.widgets.inactive = egui::style::WidgetVisuals {
        bg_fill: raised,
        weak_bg_fill: raised,
        bg_stroke: Stroke::new(1.0, border),
        corner_radius: corner,
        fg_stroke: Stroke::new(1.0, text),
        expansion: 0.0,
    };
    visuals.widgets.hovered = egui::style::WidgetVisuals {
        bg_fill: hover,
        weak_bg_fill: hover,
        bg_stroke: Stroke::new(1.0, accent),
        corner_radius: corner,
        fg_stroke: Stroke::new(1.0, text),
        expansion: 0.0,
    };
    visuals.widgets.active = egui::style::WidgetVisuals {
        bg_fill: active,
        weak_bg_fill: active,
        bg_stroke: Stroke::new(1.0, accent),
        corner_radius: corner,
        fg_stroke: Stroke::new(1.5, text),
        expansion: 0.0,
    };
    visuals.widgets.open = visuals.widgets.hovered;
    visuals.widgets.inactive.weak_bg_fill = weak;
}

/// Applies a resolved chrome theme and the configured fonts to the egui context.
pub fn apply_editor_settings(ctx: &egui::Context, theme: ThemeChoice, settings: &EditorSettings) {
    ctx.set_visuals(AppTheme::resolve(theme).visuals);

    let mut fonts = egui::FontDefinitions::default();
    // Phosphor icon font, shipped inside egui-phosphor (avoids bundling the
    // .ttf in this repo; font_bytes is egui-version-agnostic, unlike add_to_fonts).
    fonts.font_data.insert(
        "phosphor".into(),
        egui::FontData::from_static(egui_phosphor::Variant::Regular.font_bytes()).into(),
    );
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.insert(1, "phosphor".into());
    }
    if let Some(custom_font) = load_custom_font(&settings.font.family) {
        fonts.font_data.insert(
            "custom-editor-font".to_string(),
            egui::FontData::from_owned(custom_font).into(),
        );
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            family.insert(0, "custom-editor-font".to_string());
        }
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.insert(0, "custom-editor-font".to_string());
        }
    }
    ctx.set_fonts(fonts);

    // egui 0.36 keeps separate dark/light styles; keep the font sizes in sync for both.
    let font_size = settings.font.size;
    ctx.all_styles_mut(|style| {
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::monospace(font_size),
        );
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(font_size));
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::proportional(font_size),
        );
    });
}

fn load_custom_font(font_family: &str) -> Option<Vec<u8>> {
    let source = SystemSource::new();
    let handle = source
        .select_best_match(
            &[FamilyName::Title(font_family.to_string())],
            &Properties::new(),
        )
        .ok()?;
    match handle {
        Handle::Path { path, .. } => std::fs::read(path).ok(),
        Handle::Memory { bytes, .. } => Some(bytes.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choice_round_trips_through_value() {
        for choice in [ThemeChoice::Auto, ThemeChoice::Dark, ThemeChoice::Light] {
            assert_eq!(ThemeChoice::from_value(choice.value()), choice);
        }
        assert_eq!(ThemeChoice::from_value("bogus"), ThemeChoice::Auto);
    }

    #[test]
    fn auto_resolves_against_system_theme() {
        assert_eq!(
            syntax_theme_for(ThemeChoice::Auto, Some(egui::Theme::Light)),
            "base16-ocean.light"
        );
        assert_eq!(
            syntax_theme_for(ThemeChoice::Auto, Some(egui::Theme::Dark)),
            "base16-ocean.dark"
        );
        assert_eq!(
            syntax_theme_for(ThemeChoice::Auto, None),
            "base16-ocean.dark"
        );
        assert_eq!(
            syntax_theme_for(ThemeChoice::Dark, None),
            "base16-ocean.dark"
        );
        assert_eq!(
            syntax_theme_for(ThemeChoice::Light, None),
            "base16-ocean.light"
        );
    }

    #[test]
    fn resolve_never_returns_auto() {
        for choice in [ThemeChoice::Auto, ThemeChoice::Dark, ThemeChoice::Light] {
            for system in [None, Some(egui::Theme::Dark), Some(egui::Theme::Light)] {
                assert_ne!(resolve(choice, system), ThemeChoice::Auto);
            }
        }
    }

    #[test]
    fn resolved_choice_keeps_syntax_theme_mapping() {
        for system in [None, Some(egui::Theme::Dark), Some(egui::Theme::Light)] {
            assert_eq!(
                syntax_theme_for(ThemeChoice::Auto, system),
                syntax_theme_for(resolve(ThemeChoice::Auto, system), None),
            );
        }
    }

    #[test]
    fn dark_and_light_visuals_differ() {
        let dark = AppTheme::resolve(ThemeChoice::Dark);
        let light = AppTheme::resolve(ThemeChoice::Light);
        assert_ne!(dark.visuals.panel_fill, light.visuals.panel_fill);
        assert!(dark.visuals.panel_fill.r() < light.visuals.panel_fill.r());
    }
}
