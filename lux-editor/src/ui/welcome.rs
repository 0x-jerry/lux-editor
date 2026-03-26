use crate::config::Config;
use crate::events::CustomEvent;

pub fn render_welcome_view(
    ui: &mut egui::Ui,
    editor_config: &Config,
    events: &mut Vec<CustomEvent>,
) {
    ui.vertical_centered(|ui| {
        ui.add_space(100.0);
        ui.heading(egui::RichText::new("Lux Editor").size(48.0).strong());
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            ui.columns(2, |columns| {
                columns[0].vertical_centered(|ui| {
                    if ui
                        .button(egui::RichText::new("Open File").size(20.0))
                        .clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_file()
                    {
                        events.push(CustomEvent::OpenFile(path));
                    }
                });
                columns[1].vertical_centered(|ui| {
                    if ui
                        .button(egui::RichText::new("Open Folder").size(20.0))
                        .clicked()
                        && let Some(path) = rfd::FileDialog::new().pick_folder()
                    {
                        events.push(CustomEvent::OpenFolder(path));
                    }
                });
            });
        });

        ui.add_space(40.0);
        ui.separator();
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.heading("Recent Items");
            let clear_response = ui.add_enabled(
                !editor_config.recent_items.is_empty(),
                egui::Button::new("Clear"),
            );
            if clear_response.clicked() {
                events.push(CustomEvent::ClearRecentItems);
            }
        });
        ui.add_space(10.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for item in &editor_config.recent_items {
                let label = format!(
                    "{} ({})",
                    item.path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown"),
                    item.path.display()
                );
                if ui.selectable_label(false, label).clicked() {
                    if item.is_dir {
                        events.push(CustomEvent::OpenFolder(item.path.clone()));
                    } else {
                        events.push(CustomEvent::OpenFile(item.path.clone()));
                    }
                }
            }
        });
    });
}
