use eframe::egui;
use egui::{Color32, CornerRadius, Stroke, Visuals};

use super::builtin::builtin;
use super::choice::ThemeChoice;
use super::file::ThemeFile;

/// A resolved theme: chrome `Visuals` ready for `ctx.set_visuals`.
pub struct AppTheme {
    pub visuals: Visuals,
}

impl AppTheme {
    /// Visuals for a resolved choice (`Auto` falls back to dark).
    pub fn resolve(theme: ThemeChoice) -> Self {
        let file = builtin(theme);
        let base = match theme {
            ThemeChoice::Light => Visuals::light(),
            ThemeChoice::Dark | ThemeChoice::Auto => Visuals::dark(),
        };
        Self::from_file(base, file)
    }

    fn from_file(mut visuals: Visuals, file: &ThemeFile) -> Self {
        let colors = &file.colors;
        visuals.panel_fill = colors.panel_bg;
        visuals.window_fill = colors.window_bg;
        visuals.extreme_bg_color = colors.extreme_bg;
        visuals.faint_bg_color = colors.faint_bg;
        visuals.code_bg_color = colors.code_bg;
        visuals.window_stroke = Stroke::new(1.0, colors.border);
        visuals.selection.bg_fill = colors.selection_bg;
        visuals.selection.stroke = Stroke::new(1.0, colors.selection_stroke);
        visuals.hyperlink_color = colors.accent;
        visuals.weak_text_color = Some(colors.weak);
        let palette = Palette {
            text: colors.text,
            extreme_bg: colors.extreme_bg,
            border: colors.border,
            raised: colors.raised,
            hover: colors.hover,
            active: colors.active,
        };
        style_widgets(&mut visuals, &palette);
        Self { visuals }
    }
}

#[derive(Clone, Copy)]
struct Palette {
    text: Color32,
    extreme_bg: Color32,
    border: Color32,
    raised: Color32,
    hover: Color32,
    active: Color32,
}

fn style_widgets(visuals: &mut Visuals, palette: &Palette) {
    let Palette {
        text,
        extreme_bg,
        border,
        raised,
        hover,
        active,
    } = *palette;
    // Flat chrome: every interactive state is a square, borderless fill. Only
    // `noninteractive.bg_stroke` keeps a line, because egui paints panel
    // separator lines and `Frame::group` outlines from it.
    let flat = egui::style::WidgetVisuals {
        bg_fill: Color32::TRANSPARENT,
        weak_bg_fill: Color32::TRANSPARENT,
        bg_stroke: Stroke::NONE,
        corner_radius: CornerRadius::ZERO,
        fg_stroke: Stroke::new(1.0, text),
        expansion: 0.0,
    };
    visuals.widgets.noninteractive = egui::style::WidgetVisuals {
        bg_fill: raised,
        bg_stroke: Stroke::new(1.0, border),
        fg_stroke: Stroke::new(1.0, text),
        ..flat
    };
    visuals.widgets.inactive = egui::style::WidgetVisuals {
        bg_fill: raised,
        weak_bg_fill: extreme_bg,
        ..flat
    };
    visuals.widgets.hovered = egui::style::WidgetVisuals {
        bg_fill: hover,
        weak_bg_fill: hover,
        ..flat
    };
    visuals.widgets.active = egui::style::WidgetVisuals {
        bg_fill: active,
        weak_bg_fill: active,
        fg_stroke: Stroke::new(1.5, text),
        ..flat
    };
    visuals.widgets.open = visuals.widgets.hovered;
}
