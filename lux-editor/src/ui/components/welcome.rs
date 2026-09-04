use crate::config::Config;
use crate::events::{AppEvent, CustomEvent, ShellEvent};
use crate::ui::component::Component;
use eframe::egui;
use egui_phosphor::regular::{CODE, COMMAND, FILE_CODE, FOLDER, FOLDER_OPEN};

/// Welcome screen shown when no document or workspace is open.
pub struct WelcomeView;

impl Component for WelcomeView {
    type Message = CustomEvent;
    type Input<'a> = &'a Config;

    fn render(&mut self, ui: &mut egui::Ui, config: Self::Input<'_>) -> Vec<CustomEvent> {
        let mut events = Vec::new();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Hero: centered logo + title + tagline.
                // `vertical_centered` only centers direct widgets, so the row
                // gets an explicit width (measured from the fonts) to center it.
                let hero_width = {
                    let measure = |ui: &egui::Ui, font: &egui::FontId, text: &str| {
                        ui.fonts_mut(|fonts| {
                            fonts
                                .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE)
                                .size()
                                .x
                        })
                    };
                    let icon_width = measure(ui, &egui::FontId::proportional(40.0), CODE);
                    let text_width =
                        measure(ui, &egui::FontId::proportional(32.0), "Welcome back to Lux").max(
                            measure(
                                ui,
                                &egui::FontId::proportional(15.0),
                                "The editor for what's next",
                            ),
                        );
                    icon_width + ui.spacing().item_spacing.x + 12.0 + text_width
                };

                ui.vertical_centered(|ui| {
                    ui.add_space(56.0);
                    ui.scope(|ui| {
                        ui.set_width(hero_width);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(CODE)
                                    .size(40.0)
                                    .color(ui.visuals().hyperlink_color),
                            );
                            ui.add_space(12.0);
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Welcome back to Lux").size(32.0));
                                ui.label(
                                    egui::RichText::new("The editor for what's next")
                                        .size(15.0)
                                        .italics()
                                        .color(ui.visuals().weak_text_color()),
                                );
                            });
                        });
                    });
                    ui.add_space(44.0);
                });

                // Centered content column; rows are left-aligned within it.
                const CONTENT_WIDTH: f32 = 620.0;
                ui.vertical_centered(|ui| {
                    ui.scope(|ui| {
                        ui.set_width(CONTENT_WIDTH);

                        section_header(ui, "GET STARTED");
                        ui.add_space(8.0);
                        if row(ui, FILE_CODE, "Open File").clicked()
                            && let Some(path) = rfd::FileDialog::new().pick_file()
                        {
                            events.push(CustomEvent::App(AppEvent::OpenFile(path)));
                        }
                        if row(ui, FOLDER_OPEN, "Open Project").clicked()
                            && let Some(path) = rfd::FileDialog::new().pick_folder()
                        {
                            events.push(CustomEvent::App(AppEvent::OpenFolder(path)));
                        }
                        if row(ui, COMMAND, "Open Command Palette").clicked() {
                            events.push(CustomEvent::Shell(ShellEvent::ToggleCommandPanel));
                        }

                        ui.add_space(28.0);
                        recent_header(ui, "RECENT PROJECTS", config, &mut events);
                        ui.add_space(8.0);
                        if config.recent_items.is_empty() {
                            ui.label(
                                egui::RichText::new("No recent items")
                                    .italics()
                                    .color(ui.visuals().weak_text_color()),
                            );
                        } else {
                            for item in config.recent_items.iter().take(4) {
                                let icon = if item.is_dir { FOLDER } else { FILE_CODE };
                                let name = item
                                    .path
                                    .file_name()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or("Unknown");
                                if row(ui, icon, name).clicked() {
                                    if item.is_dir {
                                        events.push(CustomEvent::App(AppEvent::OpenFolder(
                                            item.path.clone(),
                                        )));
                                    } else {
                                        events.push(CustomEvent::App(AppEvent::OpenFile(
                                            item.path.clone(),
                                        )));
                                    }
                                }
                            }
                        }
                    });
                });
            });

        events
    }
}

/// A section label with an accent rule filling the rest of the row.
fn section_header(ui: &mut egui::Ui, text: &str) {
    let accent = ui.visuals().hyperlink_color;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(text)
                .small()
                .strong()
                .color(ui.visuals().weak_text_color()),
        );
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().line_segment(
            [rect.left_center(), rect.right_center()],
            egui::Stroke::new(1.0, accent),
        );
    });
}

/// A full-width clickable row: icon, label, with a hover highlight.
fn row(ui: &mut egui::Ui, icon: &str, label: &str) -> egui::Response {
    let row_height = 32.0;
    let font = egui::TextStyle::Body.resolve(ui.style());
    let icon_font = egui::FontId::proportional(16.0);
    let icon_width = ui.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(icon.to_string(), icon_font.clone(), egui::Color32::WHITE)
            .size()
            .x
    });
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), row_height),
        egui::Sense::click(),
    );
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(5),
            ui.visuals().widgets.hovered.weak_bg_fill,
        );
    }
    let x = rect.left() + 10.0;
    painter.text(
        egui::pos2(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        icon,
        icon_font,
        ui.visuals().hyperlink_color,
    );
    painter.text(
        egui::pos2(x + icon_width + 10.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        ui.visuals().text_color(),
    );
    response
}

/// The recent-projects header: label, accent rule and a right-aligned Clear.
fn recent_header(ui: &mut egui::Ui, text: &str, config: &Config, events: &mut Vec<CustomEvent>) {
    let accent = ui.visuals().hyperlink_color;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(text)
                .small()
                .strong()
                .color(ui.visuals().weak_text_color()),
        );
        let rule_width = (ui.available_width() - 64.0).max(0.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(rule_width, 1.0), egui::Sense::hover());
        ui.painter().line_segment(
            [rect.left_center(), rect.right_center()],
            egui::Stroke::new(1.0, accent),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let clear = ui.add_enabled(
                !config.recent_items.is_empty(),
                egui::Button::new("Clear").frame(false),
            );
            if clear.clicked() {
                events.push(CustomEvent::App(AppEvent::ClearRecentItems));
            }
        });
    });
}
