//! About dialog; owns its open state and closes itself.

use crate::component::Component;
use eframe::egui;

/// About dialog.
#[derive(Default)]
pub struct AboutWindow {
    open: bool,
}

impl AboutWindow {
    pub fn open(&mut self) {
        self.open = true;
    }
}

impl Component for AboutWindow {
    type Message = ();
    type Input<'a> = ();

    fn render(&mut self, ui: &mut egui::Ui, _input: Self::Input<'_>) -> Vec<Self::Message> {
        if !self.open {
            return Vec::new();
        }
        egui::Window::new("About Lux")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ui.ctx(), |ui| {
                ui.heading("Lux Editor");
                ui.label("A fast Rust editor focused on large-file performance.");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(10.0);
                if ui.button("Close").clicked() {
                    self.open = false;
                }
            });
        Vec::new()
    }
}
