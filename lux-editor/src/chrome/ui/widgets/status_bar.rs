//! Shell chrome: the bottom status bar. Sidebar toggle on the left, the
//! active tab's cursor position on the right. Top navigation lives in the
//! title bar.

use crate::component::Component;
use crate::events::{CustomEvent, ShellEvent};
use eframe::egui;
use egui_phosphor::regular::{EYE, SIDEBAR};

/// Everything the status bar needs to render itself.
pub struct StatusBarData {
    /// Sidebar open: the toggle icon is accented.
    pub sidebar_active: bool,
    /// (line, column, selection length); `None` while the active tab has no
    /// text cursor (e.g. the configuration tab).
    pub cursor: Option<(usize, usize, usize)>,
    /// `None` when the active tab is not a markdown document (no icon);
    /// otherwise whether the preview panel is open (icon accented).
    pub markdown_preview: Option<bool>,
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
                    if super::icon_button(ui, SIDEBAR, data.sidebar_active)
                        .on_hover_text("Toggle sidebar")
                        .clicked()
                    {
                        events.push(CustomEvent::Shell(ShellEvent::ToggleSidebar));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
