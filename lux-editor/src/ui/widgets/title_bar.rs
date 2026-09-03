//! App-rendered title bar: window drag region, integrated navigation tabs and
//! menu bar, plus platform-specific window controls.
//!
//! Platform adapter rules:
//! - macOS: the native title bar is kept transparent (`titlebar_transparent`)
//!   so the OS traffic lights remain available; our content is drawn over it
//!   and the left edge reserves space for the lights.
//! - Other platforms: the OS decorations are removed and this module renders
//!   minimize/maximize/close buttons plus a bottom-right resize handle.

use crate::app::TitleBarMenu;
use crate::ui::component::Component;
use eframe::egui;

/// Messages emitted by the title bar toward the embedding app.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TitleBarMessage {
    Menu(TitleBarMenu),
    /// A shell navigation tab was clicked, by index.
    Navigation(usize),
}

/// Content the title bar displays for the embedding app.
pub struct TitleBarData<'a> {
    pub app_title: &'a str,
    pub nav_tabs: &'a [&'a str],
    pub active_nav: usize,
}

/// Top title bar: window drag region, navigation tabs and menu bar.
pub struct TitleBar;

impl Component for TitleBar {
    type Message = TitleBarMessage;
    type Input<'a> = TitleBarData<'a>;

    fn render(&mut self, ui: &mut egui::Ui, data: Self::Input<'_>) -> Vec<Self::Message> {
        let mut messages = Vec::new();
        let mut push = |message: TitleBarMessage| messages.push(message);

        egui::Panel::top("title_bar")
            .exact_size(32.0)
            .frame(egui::Frame::default().fill(ui.style().visuals.panel_fill))
            .show(ui, |ui| {
                let ctx = ui.ctx().clone();
                // The drag region covers the whole bar; interactive widgets drawn
                // afterwards win hit-testing, so buttons and menus still work.
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

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if window_controls_enabled() {
                        ui.add_space(0.0);
                    } else {
                        // Room for the native macOS traffic lights.
                        ui.add_space(74.0);
                    }

                    ui.label(egui::RichText::new(data.app_title).strong().size(14.0));

                    for (index, tab) in data.nav_tabs.iter().enumerate() {
                        if nav_tab(ui, tab, index == data.active_nav).clicked() {
                            push(TitleBarMessage::Navigation(index));
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if window_controls_enabled() {
                            window_control_button(ui, "✕", |ctx| {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            });
                            let maximized = ctx
                                .input(|input| input.viewport().maximized)
                                .unwrap_or(false);
                            window_control_button(
                                ui,
                                if maximized { "❐" } else { "□" },
                                |ctx| {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(
                                        !maximized,
                                    ));
                                },
                            );
                            window_control_button(ui, "–", |ctx| {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            });
                        }
                        title_bar_menu_bar(ui, &mut push);
                    });
                });

                // Hairline under the bar.
                ui.painter().hline(
                    ui.max_rect().x_range(),
                    ui.max_rect().bottom(),
                    egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                );
            });

        messages
    }
}

/// A shell navigation tab: accented underline marks the active one.
fn nav_tab(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let accent = ui.visuals().hyperlink_color;
    let response = ui.add(
        egui::Button::new(egui::RichText::new(label).strong())
            .selected(selected)
            .corner_radius(egui::CornerRadius::same(4)),
    );
    if selected {
        let rect = egui::Rect::from_min_max(
            egui::pos2(response.rect.left() + 8.0, response.rect.bottom() - 2.0),
            egui::pos2(response.rect.right() - 8.0, response.rect.bottom()),
        );
        ui.painter().rect_filled(rect, 0.0, accent);
    }
    response
}

fn title_bar_menu_bar(ui: &mut egui::Ui, push: &mut impl FnMut(TitleBarMessage)) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Open File…").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::OpenFile));
                ui.close();
            }
            if ui.button("Open Folder…").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::OpenFolder));
                ui.close();
            }
            ui.separator();
            if ui.button("Save").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::SaveFile));
                ui.close();
            }
        });
        ui.menu_button("Edit", |ui| {
            if ui.button("Undo").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::Undo));
                ui.close();
            }
            if ui.button("Redo").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::Redo));
                ui.close();
            }
            ui.separator();
            if ui.button("Cut").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::Cut));
                ui.close();
            }
            if ui.button("Copy").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::Copy));
                ui.close();
            }
            if ui.button("Paste").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::Paste));
                ui.close();
            }
            ui.separator();
            if ui.button("Select All").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::SelectAll));
                ui.close();
            }
        });
        ui.menu_button("View", |ui| {
            if ui.button("Command Palette").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::CommandPalette));
                ui.close();
            }
            ui.separator();
            if ui.button("Editor").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::SwitchToEditor));
                ui.close();
            }
            if ui.button("Configuration").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::SwitchToConfiguration));
                ui.close();
            }
            if ui.button("Toggle Sidebar").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::ToggleSidebar));
                ui.close();
            }
        });
        ui.menu_button("Help", |ui| {
            if ui.button("About Lux").clicked() {
                push(TitleBarMessage::Menu(TitleBarMenu::About));
                ui.close();
            }
        });
    });
}

fn window_control_button(ui: &mut egui::Ui, glyph: &str, action: impl FnOnce(&egui::Context)) {
    if ui.add(egui::Button::new(glyph).frame(false)).clicked() {
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
            "⌟",
            egui::FontId::proportional(12.0),
            ui.visuals().weak_text_color(),
        );
    }
}
