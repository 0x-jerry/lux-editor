//! Shell chrome: the bottom status bar. Top navigation lives in the title bar.

use crate::ui::component::Component;
use eframe::egui;

/// The mode-specific content of the status bar.
pub enum StatusBarSection<'a> {
    Editor {
        caret_line: usize,
        caret_column: usize,
        selection_len: usize,
        document_dirty: bool,
        document_status: Option<&'a str>,
    },
    Configuration {
        config_status: Option<&'a str>,
    },
}

/// Everything the status bar needs to render itself.
pub struct StatusBarData<'a> {
    pub mode_label: &'a str,
    pub section: StatusBarSection<'a>,
    /// Right-aligned context, e.g. the active file path.
    pub right_label: &'a str,
}

/// Bottom status bar. Emits no messages; it only displays state.
pub struct StatusBar;

impl Component for StatusBar {
    type Message = ();
    type Input<'a> = StatusBarData<'a>;

    fn render(&mut self, ui: &mut egui::Ui, data: Self::Input<'_>) -> Vec<Self::Message> {
        egui::Panel::bottom("status_bar")
            .exact_size(24.0)
            .show(ui, |ui| {
                let fill = ui.visuals().widgets.noninteractive.bg_fill;
                let accent = ui.visuals().hyperlink_color;
                egui::Frame::default().fill(fill).show(ui, |ui| {
                    ui.painter().hline(
                        ui.max_rect().x_range(),
                        ui.max_rect().top(),
                        egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                    );
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(data.mode_label).strong().color(accent));
                        ui.separator();
                        match data.section {
                            StatusBarSection::Editor {
                                caret_line,
                                caret_column,
                                selection_len,
                                document_dirty,
                                document_status,
                            } => {
                                ui.label(format!(
                                    "Ln {}, Col {}  Sel {}",
                                    caret_line, caret_column, selection_len
                                ));
                                ui.separator();
                                if document_dirty {
                                    ui.label(egui::RichText::new("● Modified").color(accent));
                                } else {
                                    ui.label("Saved");
                                }
                                if let Some(status) = document_status {
                                    ui.separator();
                                    ui.label(
                                        egui::RichText::new(status)
                                            .color(ui.visuals().weak_text_color()),
                                    );
                                }
                            }
                            StatusBarSection::Configuration { config_status } => {
                                ui.label("Configuration View");
                                if let Some(status) = config_status {
                                    ui.separator();
                                    ui.label(
                                        egui::RichText::new(status)
                                            .color(ui.visuals().weak_text_color()),
                                    );
                                }
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(data.right_label)
                                    .color(ui.visuals().weak_text_color()),
                            );
                        });
                    });
                });
            });

        Vec::new()
    }
}
