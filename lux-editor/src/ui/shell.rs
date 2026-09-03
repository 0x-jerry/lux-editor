use crate::events::CustomEvent;
use crate::file_tree::FileTree;
use eframe::egui;
use std::path::Path;

pub fn draw_file_tree_panel(
    ctx: &egui::Context,
    tree: &FileTree,
    active_file_path: Option<&Path>,
    reveal_active_in_tree: bool,
    events: &mut Vec<CustomEvent>,
) {
    egui::SidePanel::left("file_tree")
        .resizable(true)
        .default_width(200.0)
        .width_range(100.0..=500.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(event) = tree.show(ui, active_file_path, reveal_active_in_tree) {
                        events.push(event);
                    }
                });
        });
}