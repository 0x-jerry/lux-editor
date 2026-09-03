use super::metrics::TextEditorMetrics;
use crate::config::Config;
use crate::events::CustomEvent;
use crate::language::HighlightSnapshot;
use crate::ui::highlight::build_highlighted_line_job;
use lux_core::Buffer;
use eframe::egui;
use std::ops::Range;

pub struct VisibleRow {
    pub index: usize,
    pub top: f32,
    pub bottom: f32,
}

pub struct RowRenderOutput {
    pub inner_rect: egui::Rect,
    pub visible_rows: Vec<VisibleRow>,
}

#[derive(Clone, Default)]
struct RevealState {
    last_caret_line: usize,
    offset: f32,
    target_offset: Option<f32>,
}

#[allow(clippy::too_many_arguments)]
pub fn render_rows(
    ui: &mut egui::Ui,
    buffer: &Buffer,
    highlight_snapshot: &HighlightSnapshot,
    editor_config: &Config,
    carets: &[(usize, usize)],
    selection_ranges: &[Range<usize>],
    active_caret_index: usize,
    caret_visible: bool,
    metrics: &TextEditorMetrics,
    events: &mut Vec<CustomEvent>,
) -> RowRenderOutput {
    // show_rows virtualizes on `row_height + item_spacing.y`; a contiguous
    // grid of lines (no gaps) is what a code editor wants, so zero the spacing
    // before the scroll area reads it.
    ui.spacing_mut().item_spacing.y = 0.0;

    let total_lines = buffer.len_lines();
    let mut visible_rows = Vec::new();

    let (active_line, _) = carets.get(active_caret_index).copied().unwrap_or((1, 1));
    let reveal_id = egui::Id::new("editor_text_editor_reveal");
    let mut reveal: RevealState = ui.data_mut(|data| data.get_temp(reveal_id).unwrap_or_default());

    // Don't fight the user dragging the scrollbar: only reveal on caret moves.
    let user_scrolling = ui.input(|input| {
        input.smooth_scroll_delta.y.abs() > 0.0 || input.raw_scroll_delta.y.abs() > 0.0
    });
    if user_scrolling {
        reveal.last_caret_line = active_line;
    }
    let viewport_height = ui.available_height();
    let caret_row_top = active_line.saturating_sub(1) as f32 * metrics.row_height;
    if !user_scrolling && reveal.last_caret_line != active_line {
        reveal.last_caret_line = active_line;
        let margin = metrics.row_height * 3.0;
        let content_height = total_lines as f32 * metrics.row_height;
        let max_scroll = (content_height - viewport_height).max(0.0);
        let caret_bottom = caret_row_top + metrics.row_height;
        if caret_row_top < reveal.offset || caret_bottom > reveal.offset + viewport_height {
            reveal.target_offset = Some((caret_row_top - margin).clamp(0.0, max_scroll));
        } else {
            reveal.target_offset = None;
        }
    }

    let mut scroll_area = egui::ScrollArea::both()
        .id_salt("editor_text_editor_scroll")
        .scroll_source(egui::scroll_area::ScrollSource::MOUSE_WHEEL)
        .auto_shrink([false, false]);
    if let Some(target) = reveal.target_offset {
        scroll_area = scroll_area.vertical_scroll_offset(target);
    }

    let scroll_output = scroll_area.show_rows(ui, metrics.row_height, total_lines, |ui, row_range| {
        for line_index in row_range {
            render_row(
                ui,
                buffer,
                highlight_snapshot,
                editor_config,
                line_index,
                carets,
                selection_ranges,
                active_caret_index,
                caret_visible,
                metrics,
                events,
                &mut visible_rows,
            );
        }
    });

    reveal.offset = scroll_output.state.offset.y;
    reveal.target_offset = None;
    ui.data_mut(|data| data.insert_temp(reveal_id, reveal));

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
    carets: &[(usize, usize)],
    selection_ranges: &[Range<usize>],
    active_caret_index: usize,
    caret_visible: bool,
    metrics: &TextEditorMetrics,
    events: &mut Vec<CustomEvent>,
    visible_rows: &mut Vec<VisibleRow>,
) {
    let line_start = buffer.text().line_to_char(line_index);
    let line_text_owned = buffer
        .line(line_index)
        .and_then(|mut lines| lines.next())
        .map(|line| line.to_string())
        .unwrap_or_default();
    let line_text = display_line_text(&line_text_owned);
    let line_len = line_text.chars().count();

    let tokens = highlight_snapshot.line_tokens.get(line_index);
    let default_color = crate::ui::highlight::snapshot_color(
        highlight_snapshot.foreground,
        ui.visuals().text_color(),
    );
    let job = build_highlighted_line_job(
        line_text,
        tokens.map(Vec::as_slice).unwrap_or(&[]),
        editor_config.settings.font.size,
        default_color,
    );
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let row_width = galley.size().x.max(ui.available_width());
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(row_width, metrics.row_height),
        egui::Sense::click_and_drag(),
    );
    let text_origin = egui::pos2(rect.left() + metrics.gutter_total_width, rect.top());

    for range in selection_ranges {
        paint_selection(
            ui,
            line_start,
            line_len,
            range,
            &galley,
            text_origin,
            metrics,
        );
    }
    ui.painter()
        .galley(text_origin, galley.clone(), ui.visuals().text_color());
    push_pointer_events(
        ui,
        line_index,
        &galley,
        text_origin,
        &rect,
        metrics,
        &response,
        events,
    );
    for (index, (caret_line, caret_column)) in carets.iter().enumerate() {
        if *caret_line != line_index + 1 {
            continue;
        }
        let char_pos = line_start + caret_column.saturating_sub(1);
        let covered = selection_ranges
            .iter()
            .any(|range| char_pos >= range.start && char_pos < range.end);
        if covered {
            continue;
        }
        let show = caret_visible || index != active_caret_index;
        paint_caret(ui, &galley, text_origin, *caret_column, show, metrics);
    }

    visible_rows.push(VisibleRow {
        index: line_index,
        top: rect.top(),
        bottom: rect.bottom(),
    });
}

fn display_line_text(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn paint_selection(
    ui: &egui::Ui,
    line_start: usize,
    line_len: usize,
    range: &Range<usize>,
    galley: &egui::text::Galley,
    text_origin: egui::Pos2,
    metrics: &TextEditorMetrics,
) {
    let line_end = line_start + line_len;
    if range.end <= line_start || range.start >= line_end {
        return;
    }
    let covered_start = range.start.max(line_start);
    let covered_end = range.end.min(line_end);
    if covered_end <= covered_start {
        return;
    }
    let x0 = if range.start <= line_start {
        text_origin.x
    } else {
        text_origin.x + galley.pos_from_cursor(egui::text::CCursor::new(covered_start - line_start)).min.x
    };
    let x1 = if range.end >= line_end {
        text_origin.x + galley.size().x
    } else {
        text_origin.x + galley.pos_from_cursor(egui::text::CCursor::new(covered_end - line_start)).min.x
    };
    let rect = egui::Rect::from_min_max(
        egui::pos2(x0, text_origin.y),
        egui::pos2(x1, text_origin.y + metrics.row_height),
    );
    ui.painter()
        .rect_filled(rect, 0.0, ui.visuals().selection.bg_fill);
}

fn paint_caret(
    ui: &egui::Ui,
    galley: &egui::text::Galley,
    text_origin: egui::Pos2,
    caret_column: usize,
    show: bool,
    metrics: &TextEditorMetrics,
) {
    if !show {
        return;
    }
    let column = caret_column.saturating_sub(1);
    let x = text_origin.x + galley.pos_from_cursor(egui::text::CCursor::new(column)).min.x;
    ui.painter().line_segment(
        [
            egui::pos2(x, text_origin.y),
            egui::pos2(x, text_origin.y + metrics.row_height),
        ],
        egui::Stroke::new(1.5, ui.visuals().text_color()),
    );
}

#[allow(clippy::too_many_arguments)]
fn push_pointer_events(
    ui: &egui::Ui,
    line_index: usize,
    galley: &egui::text::Galley,
    text_origin: egui::Pos2,
    rect: &egui::Rect,
    metrics: &TextEditorMetrics,
    response: &egui::Response,
    events: &mut Vec<CustomEvent>,
) {
    let column_from = |pointer: egui::Pos2| {
        galley
            .cursor_from_pos(egui::vec2(
                pointer.x - text_origin.x,
                pointer.y - text_origin.y,
            ))
            .index
    };

    if response.double_clicked_by(egui::PointerButton::Primary)
        && let Some(pointer) = response.interact_pointer_pos()
    {
        events.push(CustomEvent::SelectWordFromPointer {
            line_index,
            column: column_from(pointer),
        });
    }

    if response.clicked_by(egui::PointerButton::Primary)
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let selecting = ui.input(|input| input.modifiers.shift);
        let add_cursor = ui.input(|input| input.modifiers.command || input.modifiers.ctrl);
        events.push(CustomEvent::SetCaretFromPointer {
            line_index,
            column: column_from(pointer),
            selecting,
            add_cursor,
        });
    }

    if response.dragged_by(egui::PointerButton::Primary)
        && let Some(pointer) = response.interact_pointer_pos()
    {
        // Drags keep targeting the row where the press started, so project the
        // pointer onto the row it is over and let the app clamp to real lines.
        let row_delta = ((pointer.y - rect.top()) / metrics.row_height).floor() as isize;
        let pointed_line = (line_index as isize + row_delta).max(0) as usize;
        let pointed_top = rect.top() + row_delta as f32 * metrics.row_height;
        let column = galley
            .cursor_from_pos(egui::vec2(
                pointer.x - text_origin.x,
                pointer.y - pointed_top,
            ))
            .index;
        events.push(CustomEvent::SetCaretFromPointer {
            line_index: pointed_line,
            column,
            selecting: true,
            add_cursor: false,
        });
    }
}