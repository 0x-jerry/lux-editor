use crate::app::ShellView;
use crate::events::CustomEvent;
use crate::file_tree::FileTree;
use std::path::Path;

pub fn draw_shell_navigation(
    ctx: &egui::Context,
    shell_view: ShellView,
    events: &mut Vec<CustomEvent>,
) {
    egui::TopBottomPanel::top("shell_navigation").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(shell_view == ShellView::Editor, "Editor")
                .clicked()
            {
                events.push(CustomEvent::SwitchToEditor);
            }

            if ui
                .selectable_label(shell_view == ShellView::Configuration, "Configuration")
                .clicked()
            {
                events.push(CustomEvent::SwitchToConfiguration);
            }
        });
    });
}

#[allow(clippy::too_many_arguments)]
pub fn draw_status_bar(
    ctx: &egui::Context,
    shell_view: ShellView,
    caret_line: usize,
    caret_column: usize,
    selection_len: usize,
    document_dirty: bool,
    document_status: Option<&str>,
    config_status: Option<&str>,
) {
    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(24.0)
        .show(ctx, |ui| {
            let fill = ui.visuals().widgets.noninteractive.bg_fill;
            egui::Frame::default().fill(fill).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mode_label = if shell_view == ShellView::Editor {
                        "EDITOR"
                    } else {
                        "CONFIGURATION"
                    };
                    ui.label(mode_label);
                    ui.separator();
                    if shell_view == ShellView::Editor {
                        ui.label(format!(
                            "Ln {}, Col {}  Sel {}",
                            caret_line, caret_column, selection_len
                        ));
                        ui.separator();
                        ui.label(if document_dirty { "Modified" } else { "Saved" });
                        if let Some(status) = document_status {
                            ui.separator();
                            ui.label(status);
                        }
                    } else {
                        ui.label("Configuration View");
                        if let Some(status) = config_status {
                            ui.separator();
                            ui.label(status);
                        }
                    }
                });
            });
        });
}

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
