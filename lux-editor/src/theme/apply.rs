//! Pushes a resolved theme (see [`crate::theme`]) plus the configured fonts
//! onto the egui context.

use crate::settings::EditorSettings;
use crate::theme::{self, ThemeChoice};
use eframe::egui;
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use std::sync::mpsc::{Receiver, TryRecvError};

/// Background-started load of the startup font family. System font lookup
/// initializes CoreText on first call and reads a multi-MB font file — too
/// much to sit on the UI thread before the first frame, so `main` starts the
/// loader thread before the window exists and the app folds the bytes in
/// once they land (falling back to system fonts until then).
pub struct StartupFont {
    pub family: String,
    rx: Option<Receiver<Option<Vec<u8>>>>,
    resolved: Option<Option<Vec<u8>>>,
}

impl StartupFont {
    pub fn spawn(family: String) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let loader_family = family.clone();
        std::thread::spawn(move || {
            let _ = tx.send(load_custom_font(&loader_family));
        });
        Self {
            family,
            rx: Some(rx),
            resolved: None,
        }
    }

    /// `None` while the loader thread is still working; `Some(bytes-or-none)`
    /// once, taking ownership of the resolved bytes out of the loader.
    pub fn poll(&mut self) -> Option<Option<Vec<u8>>> {
        if self.resolved.is_none() {
            match self.rx.as_ref().map(Receiver::try_recv) {
                Some(Ok(bytes)) => {
                    self.rx = None;
                    self.resolved = Some(bytes);
                }
                Some(Err(TryRecvError::Disconnected)) => {
                    self.rx = None;
                    self.resolved = Some(None);
                }
                _ => return None,
            }
        }
        self.resolved.take()
    }
}

/// How `apply_editor_settings` obtains the custom editor font.
pub enum CustomFont {
    /// The startup loader hasn't landed yet: render with system fallbacks;
    /// the caller re-applies once it does.
    Pending,
    /// Preloaded on the background thread (`None` = definitively not installed).
    Preloaded(Option<Vec<u8>>),
    /// No preload applies (e.g. a config switch to another family): do the
    /// synchronous lookup.
    Sync,
}

/// Applies a resolved chrome theme and the configured fonts to the egui
/// context.
pub fn apply_editor_settings(
    ctx: &egui::Context,
    theme_choice: ThemeChoice,
    settings: &EditorSettings,
    font: CustomFont,
) {
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
    let custom_font = match font {
        CustomFont::Pending => None,
        CustomFont::Preloaded(bytes) => {
            crate::app::startup::stage_once!("startup font folded in");
            bytes.map(egui::FontData::from_owned)
        }
        CustomFont::Sync => {
            load_custom_font(&settings.font.family).map(egui::FontData::from_owned)
        }
    };
    if let Some(custom_font) = custom_font {
        fonts.font_data.insert(
            "custom-editor-font".to_string(),
            custom_font.into(),
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
