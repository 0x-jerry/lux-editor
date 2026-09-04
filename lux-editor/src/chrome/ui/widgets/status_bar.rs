//! Shell chrome: the bottom status bar. Top navigation lives in the title bar.

use crate::events::{CustomEvent, ShellEvent};
use crate::component::Component;
use eframe::egui;
use egui_phosphor::regular::{DOT, SIDEBAR};

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

/// Bottom status bar: sidebar toggle plus display-only state.
pub struct StatusBar;

impl Component for StatusBar {
    type Message = CustomEvent;
    type Input<'a> = StatusBarData<'a>;

    fn render(&mut self, ui: &mut egui::Ui, data: Self::Input<'_>) -> Vec<Self::Message> {
        let fill = ui.visuals().widgets.noninteractive.bg_fill;
        let mut events = Vec::new();
        egui::Panel::bottom("status_bar")
            .exact_size(24.0)
            .frame(
                egui::Frame::default()
                    .fill(fill)
                    .inner_margin(egui::Margin::symmetric(16, 0)),
            )
            .show(ui, |ui| {
                let accent = ui.visuals().hyperlink_color;
                ui.horizontal_centered(|ui| {
                    if super::icon_button(ui, SIDEBAR)
                        .on_hover_text("Toggle sidebar")
                        .clicked()
                    {
                        events.push(CustomEvent::Shell(ShellEvent::ToggleSidebar));
                    }
                    ui.separator();
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
                                ui.label(
                                    egui::RichText::new(format!("{DOT} Modified")).color(accent),
                                );
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

        events
    }
}
