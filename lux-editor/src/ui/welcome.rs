use crate::config::Config;
use crate::events::CustomEvent;
use eframe::egui;
use egui_phosphor::regular::{CLOCK, CODE, FILE_CODE, FOLDER_OPEN};

pub fn render_welcome_view(
    ui: &mut egui::Ui,
    editor_config: &Config,
    events: &mut Vec<CustomEvent>,
) {
    let accent = ui.visuals().hyperlink_color;
    let weak = ui.visuals().weak_text_color();

    ui.vertical_centered(|ui| {
        ui.add_space(64.0);
        ui.label(egui::RichText::new(CODE).size(48.0).color(accent));
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("Lux Editor")
                .size(38.0)
                .strong()
                .color(ui.visuals().text_color()),
        );
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "A fast Rust editor focused on large-file performance and \
                 low-latency editing.",
            )
            .size(14.0)
            .color(weak),
        );
        ui.add_space(28.0);

        ui.horizontal(|ui| {
            if big_button(ui, "Open File").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_file()
            {
                events.push(CustomEvent::OpenFile(path));
            }
            if big_button(ui, "Open Folder").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_folder()
            {
                events.push(CustomEvent::OpenFolder(path));
            }
        });

        ui.add_space(36.0);
        ui.separator();
        ui.add_space(16.0);

        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::symmetric(20, 12))
            .show(ui, |ui| {
                ui.set_width(640.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(CLOCK).color(accent));
                    ui.heading("Recent Items");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let clear = ui.add_enabled(
                            !editor_config.recent_items.is_empty(),
                            egui::Button::new("Clear").frame(false),
                        );
                        if clear.clicked() {
                            events.push(CustomEvent::ClearRecentItems);
                        }
                    });
                });
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                let scroll_height = ui.available_height().max(120.0);
                egui::ScrollArea::vertical()
                    .max_height(scroll_height)
                    .show(ui, |ui| {
                        for item in &editor_config.recent_items {
                            let icon = if item.is_dir { FOLDER_OPEN } else { FILE_CODE };
                            let name = item
                                .path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("Unknown");
                            let label = format!("{} {}    {}", icon, name, item.path.display());
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
    });
}

fn big_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).size(15.0))
            .min_size(egui::vec2(140.0, 34.0))
            .corner_radius(egui::CornerRadius::same(6)),
    )
}