use super::App;
use crate::config::Config;
use eframe::egui;
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

impl App {
    pub(super) fn apply_editor_settings(&mut self, ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        if let Some(custom_font) = Self::load_custom_font(&self.editor_config.settings.font.family)
        {
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

        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::monospace(self.editor_config.settings.font.size),
        );
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::proportional(self.editor_config.settings.font.size),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::proportional(self.editor_config.settings.font.size),
        );
        ctx.set_style(style);
    }

    fn load_custom_font(font_family: &str) -> Option<Vec<u8>> {
        let source = SystemSource::new();
        let handle = source
            .select_best_match(&[FamilyName::Title(font_family.to_string())], &Properties::new())
            .ok()?;
        match handle {
            Handle::Path { path, .. } => std::fs::read(path).ok(),
            Handle::Memory { bytes, .. } => Some(bytes.to_vec()),
        }
    }

    pub(super) fn restart_settings_watcher(&mut self) {
        let watch_roots = Config::settings_watch_roots(self.workspace_path.as_deref());
        self.settings_watcher = Self::start_settings_watcher(&watch_roots, self.event_tx.clone());
    }
}
