use super::super::pointer::pointer_to_line_column;
use super::metrics::TextAreaMetrics;
use crate::config::Config;
use crate::events::CustomEvent;
use crate::language::HighlightSnapshot;
use crate::ui::highlight::build_highlighted_line_job;
use lux_core::Buffer;

pub struct VisibleRow {
    pub index: usize,
    pub top: f32,
    pub bottom: f32,
}

pub struct RowRenderOutput {
    pub inner_rect: egui::Rect,
    pub visible_rows: Vec<VisibleRow>,
}

#[allow(clippy::too_many_arguments)]
pub fn render_rows(
    ui: &mut egui::Ui,
    buffer: &Buffer,
    highlight_snapshot: &HighlightSnapshot,
    editor_config: &Config,
    caret_line: usize,
    caret_column: usize,
    selection_len: usize,
    caret_visible: bool,
    metrics: &TextAreaMetrics,
    events: &mut Vec<CustomEvent>,
) -> RowRenderOutput {
    let total_lines = buffer.len_lines();
    let mut visible_rows = Vec::new();

    let scroll_output = egui::ScrollArea::both()
        .id_salt("editor_text_area_scroll")
        .scroll_source(egui::scroll_area::ScrollSource::MOUSE_WHEEL)
        .auto_shrink([false, false])
        .show_rows(ui, metrics.row_height, total_lines, |ui, row_range| {
            for line_index in row_range {
                render_row(
                    ui,
                    buffer,
                    highlight_snapshot,
                    editor_config,
                    line_index,
                    caret_line,
                    caret_column,
                    selection_len,
                    caret_visible,
                    metrics,
                    events,
                    &mut visible_rows,
                );
            }
        });

    RowRenderOutput {
        inner_rect: scroll_output.inner_rect,
        visible_rows,
    }
}

#[allow(clippy::too_many_arguments)]
fn render_row(
    ui: &mut egui::Ui,
    buffer: &Buffer,
    highlight_snapshot: &HighlightSnapshot,
    editor_config: &Config,
    line_index: usize,
    caret_line: usize,
    caret_column: usize,
    selection_len: usize,
    caret_visible: bool,
    metrics: &TextAreaMetrics,
    events: &mut Vec<CustomEvent>,
    visible_rows: &mut Vec<VisibleRow>,
) {
    if let Some(mut lines_iter) = buffer.line(line_index)
        && let Some(line) = lines_iter.next()
    {
        let line_text_owned = line.to_string();
        let line_text = display_line_text(&line_text_owned);
        let response = ui
            .horizontal(|ui| {
                ui.add_space(metrics.gutter_total_width);

                if let Some(line_tokens) = highlight_snapshot.line_tokens.get(line_index) {
                    let job = build_highlighted_line_job(
                        line_text,
                        line_tokens,
                        editor_config.settings.font.size,
                    );
                    ui.add(egui::Label::new(job).sense(egui::Sense::click_and_drag()))
                } else {
                    ui.add(egui::Label::new(line_text).sense(egui::Sense::click_and_drag()))
                }
            })
            .inner;

        visible_rows.push(VisibleRow {
            index: line_index,
            top: response.rect.top(),
            bottom: response.rect.bottom(),
        });

        push_pointer_events(ui, line_index, &response, metrics, events);
        paint_caret(
            ui,
            response.rect,
            line_index,
            caret_line,
            caret_column,
            selection_len,
            caret_visible,
            metrics,
        );
    }
}

fn display_line_text(line_text: &str) -> &str {
    line_text.trim_end_matches(['\r', '\n'])
}

fn push_pointer_events(
    ui: &egui::Ui,
    line_index: usize,
    response: &egui::Response,
    metrics: &TextAreaMetrics,
    events: &mut Vec<CustomEvent>,
) {
    if response.clicked_by(egui::PointerButton::Primary)
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let selecting = ui.input(|input| input.modifiers.shift);
        let (_, column) = pointer_to_line_column(
            pointer,
            response.rect,
            metrics.row_height,
            line_index,
            metrics.char_width,
        );
        events.push(CustomEvent::SetCaretFromPointer {
            line_index,
            column,
            selecting,
        });
    }

    if response.dragged_by(egui::PointerButton::Primary)
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let (drag_line_index, column) = pointer_to_line_column(
            pointer,
            response.rect,
            metrics.row_height,
            line_index,
            metrics.char_width,
        );
        events.push(CustomEvent::SetCaretFromPointer {
            line_index: drag_line_index,
            column,
            selecting: true,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_caret(
    ui: &egui::Ui,
    rect: egui::Rect,
    line_index: usize,
    caret_line: usize,
    caret_column: usize,
    selection_len: usize,
    caret_visible: bool,
    metrics: &TextAreaMetrics,
) {
    if selection_len == 0 && caret_visible && line_index + 1 == caret_line {
        let x = rect.min.x + (caret_column.saturating_sub(1) as f32 * metrics.char_width);
        ui.painter().line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.5, ui.visuals().text_color()),
        );
    }
}
