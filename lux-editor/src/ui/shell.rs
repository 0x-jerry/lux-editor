use crate::events::CustomEvent;
use crate::file_tree::FileTree;
use eframe::egui;
use egui_phosphor::regular::SIDEBAR;
use std::path::Path;

pub fn draw_file_tree_panel(
    ctx: &egui::Context,
    tree: &FileTree,
    workspace_path: Option<&Path>,
    active_file_path: Option<&Path>,
    reveal_active_in_tree: bool,
    events: &mut Vec<CustomEvent>,
) {
    egui::SidePanel::left("file_tree")
        .resizable(true)
        .default_width(220.0)
        .width_range(120.0..=480.0)
        .show(ctx, |ui| {
            egui::Frame::default()
                .inner_margin(egui::Margin::symmetric(6, 6))
                .show(ui, |ui| {
                    file_tree_header(ui, workspace_path, events);
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                });
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(event) = tree.show(ui, active_file_path, reveal_active_in_tree) {
                        events.push(event);
                    }
                });
        });
}

fn file_tree_header(
    ui: &mut egui::Ui,
    workspace_path: Option<&Path>,
    events: &mut Vec<CustomEvent>,
) {
    let accent = ui.visuals().hyperlink_color;
    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::new(
                    egui::RichText::new(SIDEBAR).color(ui.visuals().weak_text_color()),
                )
                .frame(false),
            )
            .on_hover_text("Toggle sidebar")
            .clicked()
        {
            events.push(CustomEvent::ToggleSidebar);
        }
        let name = workspace_path
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Explorer");
        ui.label(egui::RichText::new(name).strong().color(accent));
    });
}