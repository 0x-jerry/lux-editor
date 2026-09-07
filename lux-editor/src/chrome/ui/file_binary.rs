use crate::component::Component;
use eframe::egui;
use egui_phosphor::regular::BINARY;
use std::path::Path;

/// Editor-area page for a tab whose file is binary: the bytes cannot be edited
/// as text, so a guide replaces the text area instead of an empty buffer that
/// could be saved over the real file.
pub struct FileBinaryView;

pub struct FileBinaryInput<'a> {
    pub path: &'a Path,
}

impl Component for FileBinaryView {
    type Message = ();
    type Input<'a> = FileBinaryInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<()> {
        let FileBinaryInput { path } = input;

        ui.vertical_centered(|ui| {
            ui.add_space(56.0);
            ui.label(
                egui::RichText::new(BINARY)
                    .size(40.0)
                    .color(ui.visuals().warn_fg_color),
            );
            ui.add_space(12.0);
            ui.label(egui::RichText::new("Binary file").size(24.0));
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(path.display().to_string())
                    .size(13.0)
                    .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("This file is binary and cannot be edited here.")
                    .size(12.0)
                    .color(ui.visuals().weak_text_color()),
            );
        });

        Vec::new()
    }
}
