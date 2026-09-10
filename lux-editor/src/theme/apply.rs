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

/// Background-started load of a font family. System font lookup initializes
/// CoreText on first call and reads a multi-MB font file — too much to sit on
/// the UI thread before the first frame, so `main` starts the loader thread
/// before the window exists and the app folds the bytes in once they land
/// (falling back to system fonts until then). A later family change goes
/// through the same path instead of blocking the style pass.
pub struct StartupFont {
    pub family: String,
    rx: Option<Receiver<Option<Vec<u8>>>>,
    resolved: Option<Option<Vec<u8>>>,
}

impl StartupFont {
    pub fn spawn(family: String) -> Self {
        // An empty family asks for no custom font at all (the system default),
        // so there is nothing to look up and no thread to start.
        if family.is_empty() {
            return Self {
                family,
                rx: None,
                resolved: Some(None),
            };
        }
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
    /// once it is done. The result is kept, so every later style pass reuses it
    /// instead of paying the lookup again.
    pub fn resolved(&mut self) -> Option<Option<Vec<u8>>> {
        if self.resolved.is_none() {
            self.resolved = match self.rx.as_ref().map(Receiver::try_recv) {
                Some(Ok(bytes)) => {
                    self.rx = None;
                    Some(bytes)
                }
                Some(Err(TryRecvError::Disconnected)) => {
                    self.rx = None;
                    Some(None)
                }
                _ => return None,
            };
        }
        self.resolved.clone()
    }
}

/// How `apply_editor_settings` obtains the custom editor font.
pub enum CustomFont {
    /// The loader hasn't landed yet: render with system fallbacks; the caller
    /// re-applies once it does.
    Pending,
    /// Loaded on the background thread (`None` = definitively not installed).
    Loaded(Option<Vec<u8>>),
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
    // Devicons glyph font (file-type icons in the sidebar tree); a dedicated
    // family so its PUA codepoints never shadow phosphor's in shared fallback.
    fonts.font_data.insert(
        "devicons".into(),
        egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/SymbolsNerdFontMono-Regular.ttf"
        ))
        .into(),
    );
    fonts.families.insert(
        egui::FontFamily::Name("devicons".into()),
        vec!["devicons".into()],
    );
    let custom_font = match font {
        CustomFont::Pending => None,
        CustomFont::Loaded(bytes) => {
            crate::app::startup::stage_once!("startup font folded in");
            bytes.map(egui::FontData::from_owned)
        }
    };
    if let Some(custom_font) = custom_font {
        fonts
            .font_data
            .insert("custom-editor-font".to_string(), custom_font.into());
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
        // Pin the floating scrollbar to its thin width so it always stays thin and never expands on hover.
        style.spacing.scroll.bar_width = style.spacing.scroll.floating_width;
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

#[cfg(test)]
mod tests {
    use super::StartupFont;

    #[test]
    fn an_empty_family_resolves_to_no_custom_font() {
        // No system lookup and no loader thread: the answer is known up front,
        // and stays known on every later style pass.
        let mut loader = StartupFont::spawn(String::new());
        assert_eq!(loader.resolved(), Some(None));
        assert_eq!(loader.resolved(), Some(None));
    }
}
