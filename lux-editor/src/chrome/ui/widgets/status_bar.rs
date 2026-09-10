//! Shell chrome: the bottom status bar. Sidebar toggle on the left, the
//! active tab's cursor position on the right. Top navigation lives in the
//! title bar.

use crate::chrome::ui::file_image::ImageStatus;
use crate::component::Component;
use crate::events::{CustomEvent, ShellEvent};
use eframe::egui;
use egui_phosphor::regular::{EYE, SIDEBAR};

/// Everything the status bar needs to render itself.
pub struct StatusBarData {
    /// `None` when there is no sidebar to toggle (no workspace open);
    /// otherwise whether it is open (the toggle icon is accented).
    pub sidebar: Option<bool>,
    /// (line, column, selection length); `None` while the active tab has no
    /// text cursor (e.g. the configuration tab).
    pub cursor: Option<(usize, usize, usize)>,
    /// `None` when the active tab is not a markdown document (no icon);
    /// otherwise whether the preview panel is open (icon accented).
    pub markdown_preview: Option<bool>,
    /// Size of the active tab's file on disk, when known.
    pub file_size: Option<u64>,
    /// The active tab is an image on display: native pixels and shown scale.
    pub image_status: Option<ImageStatus>,
}

/// Bottom status bar: sidebar toggle plus the active tab's cursor position.
pub struct StatusBar;

impl Component for StatusBar {
    type Message = CustomEvent;
    type Input<'a> = StatusBarData;

    fn render(&mut self, ui: &mut egui::Ui, data: Self::Input<'_>) -> Vec<Self::Message> {
        let fill = ui.visuals().widgets.noninteractive.bg_fill;
        // One comfortable row that grows with the theme's interact size.
        let height = ui.spacing().interact_size.y.max(24.0);
        let mut events = Vec::new();
        egui::Panel::bottom("status_bar")
            .exact_size(height)
            .frame(
                egui::Frame::default()
                    .fill(fill)
                    .inner_margin(egui::Margin::symmetric(16, 0)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    if let Some(sidebar_active) = data.sidebar
                        && super::icon_button(ui, SIDEBAR, sidebar_active)
                            .on_hover_text("Toggle sidebar")
                            .clicked()
                    {
                        events.push(CustomEvent::Shell(ShellEvent::ToggleSidebar));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(bytes) = data.file_size {
                            ui.label(format_file_size(bytes));
                        }
                        if let Some(status) = data.image_status {
                            ui.label(format!(
                                "{} × {}   {:.0}%",
                                status.pixels.x,
                                status.pixels.y,
                                status.scale * 100.0
                            ));
                        }
                        if let Some(preview_active) = data.markdown_preview
                            && super::icon_button(ui, EYE, preview_active)
                                .on_hover_text("Toggle markdown preview")
                                .clicked()
                        {
                            events.push(CustomEvent::Shell(ShellEvent::ToggleMarkdownPreview));
                        }
                        if let Some((line, column, selection_len)) = data.cursor {
                            ui.label(format!("Ln {line}, Col {column}  Sel {selection_len}"));
                        }
                    });
                });
            });

        events
    }
}

fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::format_file_size;

    #[test]
    fn file_sizes_format_in_binary_units() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(1023), "1023 B");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(format_file_size(3 * 1024 * 1024 * 1024), "3.00 GB");
    }
}
