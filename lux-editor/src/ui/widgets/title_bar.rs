//! App-rendered title bar: window drag region and platform-specific window
//! controls.
//!
//! Platform adapter rules:
//! - macOS: the native title bar is kept transparent (`titlebar_transparent`)
//!   so the OS traffic lights remain available; our content is drawn over it
//!   and the left edge reserves space for the lights.
//! - Other platforms: the OS decorations are removed and this module renders
//!   minimize/maximize/close buttons plus a bottom-right resize handle.

use crate::ui::component::Component;
use eframe::egui;
use egui_phosphor::regular::{ARROWS_IN, ARROWS_OUT, MINUS, NOTCHES, X};

/// Content the title bar displays for the embedding app.
pub struct TitleBarData<'a> {
    pub app_title: &'a str,
}

/// Top title bar: window drag region and window controls.
pub struct TitleBar;

impl Component for TitleBar {
    type Message = ();
    type Input<'a> = TitleBarData<'a>;

    fn render(&mut self, ui: &mut egui::Ui, data: Self::Input<'_>) -> Vec<Self::Message> {
        egui::Panel::top("title_bar")
            .exact_size(32.0)
            .frame(
                egui::Frame::default()
                    .fill(ui.style().visuals.panel_fill)
                    .inner_margin(egui::Margin::symmetric(4, 0)),
            )
            .show(ui, |ui| {
                let ctx = ui.ctx().clone();
                // The drag region covers the whole bar; interactive widgets drawn
                // afterwards win hit-testing, so buttons still work.
                let drag_response = ui.interact(
                    ui.max_rect(),
                    ui.id().with("title_bar_drag"),
                    egui::Sense::click_and_drag(),
                );
                if drag_response.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag_response.double_clicked() {
                    let maximized = ctx
                        .input(|input| input.viewport().maximized)
                        .unwrap_or(false);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }

                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if window_controls_enabled() {
                        ui.add_space(0.0);
                    } else {
                        // Room for the native macOS traffic lights.
                        ui.add_space(74.0);
                    }

                    ui.label(egui::RichText::new(data.app_title).strong().size(14.0));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if window_controls_enabled() {
                            window_control_button(ui, X, |ctx| {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            });
                            let maximized = ctx
                                .input(|input| input.viewport().maximized)
                                .unwrap_or(false);
                            window_control_button(
                                ui,
                                if maximized { ARROWS_IN } else { ARROWS_OUT },
                                |ctx| {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(
                                        !maximized,
                                    ));
                                },
                            );
                            window_control_button(ui, MINUS, |ctx| {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            });
                        }
                    });
                });

                // Hairline under the bar.
                ui.painter().hline(
                    ui.max_rect().x_range(),
                    ui.max_rect().bottom(),
                    egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                );
            });

        Vec::new()
    }
}

fn window_control_button(ui: &mut egui::Ui, glyph: &str, action: impl FnOnce(&egui::Context)) {
    if super::icon_button(ui, glyph).clicked() {
        action(ui.ctx());
    }
}

/// Whether this build renders its own window controls. macOS keeps the native
/// traffic lights, so only frameless platforms get custom buttons.
pub fn window_controls_enabled() -> bool {
    !cfg!(target_os = "macos")
}

/// Drag handle that resizes the frameless window from the bottom-right corner.
/// A no-op on platforms that keep native window chrome.
pub fn window_resize_handle(ui: &mut egui::Ui) {
    if !window_controls_enabled() {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::drag());
        let response = response.on_hover_and_drag_cursor(egui::CursorIcon::ResizeNwSe);
        if response.dragged() {
            let delta = response.drag_delta();
            let current = ui
                .ctx()
                .input(|input| input.viewport().inner_rect.unwrap().size());
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(current + delta));
        }
        let painter = ui.painter_at(rect);
        painter.text(
            egui::pos2(rect.right(), rect.bottom()),
            egui::Align2::RIGHT_BOTTOM,
            NOTCHES,
            egui::FontId::proportional(12.0),
            ui.visuals().weak_text_color(),
        );
    }
}
