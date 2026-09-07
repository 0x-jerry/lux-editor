use crate::component::Component;
use eframe::egui;
use egui_phosphor::regular::FILE_X;
use std::path::Path;

/// Editor-area page for a tab whose file is gone, instead of an empty buffer
/// that could be saved over the real file. The page is transient: the watcher
/// re-reads the tab as soon as the file exists again.
pub struct FileMissingView;

pub struct FileMissingInput<'a> {
    pub path: &'a Path,
    pub loader_error: Option<&'a str>,
}

impl Component for FileMissingView {
    type Message = ();
    type Input<'a> = FileMissingInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<()> {
        let FileMissingInput { path, loader_error } = input;

        ui.vertical_centered(|ui| {
            ui.add_space(56.0);
            ui.label(
                egui::RichText::new(FILE_X)
                    .size(40.0)
                    .color(ui.visuals().warn_fg_color),
            );
            ui.add_space(12.0);
            ui.label(egui::RichText::new("File not found").size(24.0));
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(path.display().to_string())
                    .size(13.0)
                    .color(ui.visuals().weak_text_color()),
            );
            if let Some(error) = loader_error {
                ui.label(
                    egui::RichText::new(error)
                        .size(12.0)
                        .italics()
                        .color(ui.visuals().weak_text_color()),
                );
            }
        });

        Vec::new()
    }
}
