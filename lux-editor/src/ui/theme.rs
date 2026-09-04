//! Pushes a resolved theme (see [`crate::theme`]) plus the configured fonts
//! onto the egui context.

use crate::config::EditorSettings;
use crate::theme::{self, ThemeChoice};
use eframe::egui;
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

/// Applies a resolved chrome theme and the configured fonts to the egui context.
pub fn apply_editor_settings(ctx: &egui::Context, theme_choice: ThemeChoice, settings: &EditorSettings) {
    ctx.set_visuals(theme::AppTheme::resolve(theme_choice).visuals);

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
        // Button-family widgets (buttons, selectable labels, checkboxes, combo
        // boxes) are the only things that honor this in egui 0.36; raw Sense
        // hit-areas set their cursor per-response instead.
        style.visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
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
