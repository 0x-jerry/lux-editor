use crate::app::ShellView;
use crate::events::CustomEvent;
use crate::file_tree::FileTree;

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

pub fn draw_file_tree_panel(ctx: &egui::Context, tree: &FileTree, events: &mut Vec<CustomEvent>) {
    egui::SidePanel::left("file_tree")
        .resizable(true)
        .default_width(200.0)
        .width_range(100.0..=500.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if let Some(event) = tree.show(ui) {
                        events.push(event);
                    }
                });
        });
}
