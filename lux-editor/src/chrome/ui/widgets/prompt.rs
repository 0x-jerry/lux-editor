//! Shared chrome for the app's modal prompts (unsaved changes, destructive
//! confirmations): a card that reads as a prompt rather than a second window.

use eframe::egui;

pub fn prompt_frame(ui: &egui::Ui) -> egui::Frame {
    egui::Frame::NONE
        .corner_radius(egui::CornerRadius::same(10))
        .fill(ui.visuals().window_fill)
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .shadow(egui::Shadow {
            offset: [0, 8],
            blur: 32,
            spread: 0,
            color: egui::Color32::from_black_alpha(90),
        })
        .inner_margin(egui::Margin::same(20))
}
