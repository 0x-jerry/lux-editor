use super::metrics::TextEditorMetrics;
use super::row::VisibleRow;
use eframe::egui;

pub fn paint_gutter(
    ui: &egui::Ui,
    inner_rect: egui::Rect,
    visible_rows: &[VisibleRow],
    caret_line: usize,
    metrics: &TextEditorMetrics,
) {
    let gutter_rect = egui::Rect::from_min_max(
        inner_rect.left_top(),
        egui::pos2(
            inner_rect.left() + metrics.gutter_total_width,
            inner_rect.bottom(),
        ),
    );

    ui.painter()
        .rect_filled(gutter_rect, 0.0, metrics.gutter_bg);

    for row in visible_rows {
        let row_rect = egui::Rect::from_min_max(
            egui::pos2(gutter_rect.left(), row.top),
            egui::pos2(gutter_rect.right(), row.bottom),
        );

        if row.index + 1 == caret_line {
            ui.painter()
                .rect_filled(row_rect, 0.0, metrics.gutter_active_bg);
        }

        ui.painter().text(
            egui::pos2(row_rect.right() - metrics.char_width, row_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            (row.index + 1).to_string(),
            metrics.gutter_font_id.clone(),
            metrics.gutter_text_color,
        );
    }

    ui.painter().line_segment(
        [
            egui::pos2(gutter_rect.right(), gutter_rect.top()),
            egui::pos2(gutter_rect.right(), gutter_rect.bottom()),
        ],
        egui::Stroke::new(1.0, metrics.gutter_separator_color),
    );
}
