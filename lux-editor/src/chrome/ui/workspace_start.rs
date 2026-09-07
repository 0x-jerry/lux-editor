use crate::component::Component;
use crate::events::{CustomEvent, ShellEvent};
use eframe::egui;
use egui_phosphor::regular::FOLDER_OPEN;
use std::path::Path;

/// Editor-area screen for an open workspace with nothing open in it.
pub struct WorkspaceStartView;

pub struct WorkspaceStartInput<'a> {
    pub workspace_path: &'a Path,
    pub sidebar_visible: bool,
}

impl Component for WorkspaceStartView {
    type Message = CustomEvent;
    type Input<'a> = WorkspaceStartInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<CustomEvent> {
        let mut events = Vec::new();
        let height = ui.available_size().y;
        let name = input
            .workspace_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace");

        ui.vertical_centered(|ui| {
            ui.add_space((height / 3.0).max(48.0));
            ui.label(
                egui::RichText::new(FOLDER_OPEN)
                    .size(36.0)
                    .color(ui.visuals().hyperlink_color),
            );
            ui.add_space(10.0);
            ui.label(egui::RichText::new(name).size(20.0).strong());
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Open a file from the file tree on the left to start editing.")
                    .weak(),
            );
            ui.add_space(20.0);
            if !input.sidebar_visible {
                let shortcut = if cfg!(target_os = "macos") {
                    "⌘B"
                } else {
                    "Ctrl+B"
                };
                if ui.button(format!("Show file tree ({shortcut})")).clicked() {
                    events.push(CustomEvent::Shell(ShellEvent::ToggleSidebar));
                }
            }
        });

        events
    }
}
