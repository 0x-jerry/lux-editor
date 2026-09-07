use super::metrics::TextEditorMetrics;
use super::row::VisibleRow;
use crate::component::Component;
use eframe::egui;

pub struct GutterInput<'a> {
    pub rect: egui::Rect,
    pub visible_rows: &'a [VisibleRow],
    pub active_line: usize,
    pub metrics: &'a TextEditorMetrics,
}

/// The line-number strip, a sibling box left of the text area. Display-only.
///
/// Paints into the caller-reserved box and clips to it, so line numbers that
/// overflow the visible rows (show_rows yields one row past the viewport) are
/// hidden instead of bleeding below the box.
pub struct Gutter;

impl Component for Gutter {
    type Message = ();
    type Input<'a> = GutterInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<Self::Message> {
        let painter = ui.painter().with_clip_rect(input.rect);
        painter.rect_filled(input.rect, 0.0, input.metrics.gutter_bg);

        for row in input.visible_rows {
            let row_rect = egui::Rect::from_min_max(
                egui::pos2(input.rect.left(), row.top),
                egui::pos2(input.rect.right(), row.bottom),
            );
            if !painter.clip_rect().intersects(row_rect) {
                continue;
            }
            if row.index + 1 == input.active_line {
                painter.rect_filled(row_rect, 0.0, input.metrics.gutter_active_bg);
            }
            painter.text(
                egui::pos2(row_rect.right() - input.metrics.char_width, row_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                (row.index + 1).to_string(),
                input.metrics.gutter_font_id.clone(),
                input.metrics.gutter_text_color,
            );
        }

        vec![]
    }
}
