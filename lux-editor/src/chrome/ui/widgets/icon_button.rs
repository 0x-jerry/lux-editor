//! Shared clickable icon glyph: faint at rest, theme accent on hover.

use eframe::egui;

/// Frameless, square icon button in the current flow layout.
pub fn icon_button(ui: &mut egui::Ui, glyph: &str) -> egui::Response {
    let side = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
    let hovered = response.hovered() || response.is_pointer_button_down_on();
    let font_id = egui::TextStyle::Button.resolve(ui.style());
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        font_id,
        icon_text_color(ui, hovered),
    );
    // Not an egui::Button, so the global `visuals.interact_cursor` does not
    // apply; set the hand cursor explicitly to match the rest of the chrome.
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn icon_text_color(ui: &egui::Ui, hovered: bool) -> egui::Color32 {
    if hovered {
        ui.visuals().hyperlink_color
    } else {
        ui.visuals().weak_text_color()
    }
}
