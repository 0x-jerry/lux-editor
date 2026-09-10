//! Save/discard prompt shown before closing a tab with unsaved changes.

use crate::chrome::ui::widgets::prompt_frame;
use crate::component::Component;
use crate::events::{CustomEvent, DocumentEvent};
use crate::tabs::TabMeta;
use eframe::egui;
use egui_phosphor::regular::WARNING_CIRCLE;

/// Which tab is waiting for a close decision, if any.
#[derive(Default)]
pub(crate) struct ClosePrompt {
    target: Option<u64>,
}

impl ClosePrompt {
    pub(crate) fn request(&mut self, id: u64) {
        self.target = Some(id);
    }

    pub(crate) fn cancel(&mut self) {
        self.target = None;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.target.is_some()
    }

    /// Whether the prompt is currently asking about `id`.
    pub(crate) fn is_open_for(&self, id: u64) -> bool {
        self.target == Some(id)
    }
}

enum CloseChoice {
    Save,
    Discard,
    Cancel,
}

/// Card width: wide enough for a file name on one line, narrow enough to read
/// as a prompt rather than a window.
const CARD_WIDTH: f32 = 400.0;
const BUTTON_WIDTH: f32 = 88.0;
const BUTTON_HEIGHT: f32 = 30.0;

impl Component for ClosePrompt {
    type Message = CustomEvent;
    type Input<'a> = &'a [TabMeta];

    fn render(&mut self, ui: &mut egui::Ui, tabs: &[TabMeta]) -> Vec<CustomEvent> {
        let Some(target) = self.target else {
            return Vec::new();
        };
        let Some(tab) = tabs.iter().find(|tab| tab.id == target) else {
            // The tab died under the prompt (workspace switch, external close).
            self.target = None;
            return Vec::new();
        };

        let mut choice = None;
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let modal = egui::Modal::new(egui::Id::new("close_prompt"))
            .frame(prompt_frame(ui))
            .show(ui.ctx(), |ui| {
                ui.set_min_width(CARD_WIDTH);
                ui.set_max_width(CARD_WIDTH);
                let weak = ui.visuals().weak_text_color();

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(WARNING_CIRCLE)
                            .size(22.0)
                            .color(ui.visuals().warn_fg_color),
                    );
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Unsaved changes").size(17.0).strong());
                });

                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(
                        egui::RichText::new("Save changes to")
                            .size(13.0)
                            .color(weak),
                    );
                    ui.label(
                        egui::RichText::new(format!("\u{201c}{}\u{201d}", tab.title))
                            .size(13.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("before closing?")
                            .size(13.0)
                            .color(weak),
                    );
                });
                ui.add_space(3.0);
                ui.label(
                    egui::RichText::new("Closing without saving discards your edits.")
                        .size(12.0)
                        .color(weak),
                );

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(12.0);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let accent = ui.visuals().hyperlink_color;
                    let save = ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Save")
                                    .strong()
                                    .color(contrasting_text(accent)),
                            )
                            .fill(accent)
                            .corner_radius(egui::CornerRadius::same(6))
                            .min_size(egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT)),
                        )
                        .on_hover_text("Save and close (\u{21a9})");
                    if save.clicked() {
                        choice = Some(CloseChoice::Save);
                    }

                    let discard = ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Discard").color(ui.visuals().error_fg_color),
                            )
                            .corner_radius(egui::CornerRadius::same(6))
                            .min_size(egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT)),
                        )
                        .on_hover_text("Close without saving");
                    if discard.clicked() {
                        choice = Some(CloseChoice::Discard);
                    }

                    let cancel = ui
                        .add(
                            egui::Button::new("Cancel")
                                .corner_radius(egui::CornerRadius::same(6))
                                .min_size(egui::vec2(BUTTON_WIDTH, BUTTON_HEIGHT)),
                        )
                        .on_hover_text("Keep editing (Esc)");
                    if cancel.clicked() {
                        choice = Some(CloseChoice::Cancel);
                    }
                });
            });

        if choice.is_none() {
            if modal.should_close() {
                choice = Some(CloseChoice::Cancel);
            } else if enter {
                choice = Some(CloseChoice::Save);
            }
        }

        let mut events = Vec::new();
        match choice {
            Some(CloseChoice::Save) => {
                self.target = None;
                events.push(CustomEvent::Document(DocumentEvent::SaveAndCloseTab(
                    target,
                )));
            }
            Some(CloseChoice::Discard) => {
                self.target = None;
                events.push(CustomEvent::Document(DocumentEvent::DiscardTab(target)));
            }
            Some(CloseChoice::Cancel) => self.target = None,
            None => {}
        }
        events
    }
}

/// Black or white, whichever reads on `color`; the theme accent is a pale blue
/// on the dark theme and a deep blue on the light one.
fn contrasting_text(color: egui::Color32) -> egui::Color32 {
    let [r, g, b, _] = color.to_array();
    let luminance = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
    if luminance > 140.0 {
        egui::Color32::BLACK
    } else {
        egui::Color32::WHITE
    }
}

#[cfg(test)]
mod tests {
    use super::contrasting_text;
    use eframe::egui;

    #[test]
    fn primary_button_text_contrasts_with_the_accent() {
        // Dark theme accent (#6aa1ff) is pale: black text.
        assert_eq!(
            contrasting_text(egui::Color32::from_rgb(0x6a, 0xa1, 0xff)),
            egui::Color32::BLACK
        );
        // Light theme accent (#0b6bcb) is deep: white text.
        assert_eq!(
            contrasting_text(egui::Color32::from_rgb(0x0b, 0x6b, 0xcb)),
            egui::Color32::WHITE
        );
    }
}
