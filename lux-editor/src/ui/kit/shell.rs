//! Shell chrome: top navigation bar and bottom status bar.

use eframe::egui;

/// A selectable tab in the shell navigation bar.
#[derive(Clone, Debug)]
pub struct NavigationTab {
    pub label: String,
}

impl NavigationTab {
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into() }
    }
}

/// Renders the top navigation bar.
///
/// Returns `Some(index)` when the tab at `index` was clicked by the user.
pub fn navigation(ctx: &egui::Context, tabs: &[NavigationTab], active: usize) -> Option<usize> {
    let mut clicked = None;
    egui::TopBottomPanel::top("shell_navigation").show(ctx, |ui| {
        ui.horizontal(|ui| {
            for (index, tab) in tabs.iter().enumerate() {
                if ui.selectable_label(active == index, &tab.label).clicked() {
                    clicked = Some(index);
                }
            }
        });
    });
    clicked
}

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
}

/// Renders the bottom status bar.
pub fn status_bar(ctx: &egui::Context, data: StatusBarData<'_>) {
    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(24.0)
        .show(ctx, |ui| {
            let fill = ui.visuals().widgets.noninteractive.bg_fill;
            egui::Frame::default().fill(fill).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(data.mode_label);
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
                            ui.label(if document_dirty { "Modified" } else { "Saved" });
                            if let Some(status) = document_status {
                                ui.separator();
                                ui.label(status);
                            }
                        }
                        StatusBarSection::Configuration { config_status } => {
                            ui.label("Configuration View");
                            if let Some(status) = config_status {
                                ui.separator();
                                ui.label(status);
                            }
                        }
                    }
                });
            });
        });
}