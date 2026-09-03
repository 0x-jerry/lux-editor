mod gutter;
mod metrics;
mod row;

use crate::config::Config;
use crate::events::CustomEvent;
use crate::language::HighlightSnapshot;
use lux_core::Buffer;
use eframe::egui;
use std::ops::Range;

use self::gutter::paint_gutter;
use self::metrics::measure_text_editor;
use self::row::render_rows;

pub struct TextEditorState<'a> {
    pub buffer: &'a Buffer,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    /// All cursor positions as 1-based (line, column).
    pub carets: &'a [(usize, usize)],
    pub selection_ranges: &'a [Range<usize>],
    pub active_caret_index: usize,
    pub caret_visible: bool,
}

pub fn render_text_editor(
    ui: &mut egui::Ui,
    state: TextEditorState<'_>,
    events: &mut Vec<CustomEvent>,
) {
    let TextEditorState {
        buffer,
        highlight_snapshot,
        editor_config,
        carets,
        selection_ranges,
        active_caret_index,
        caret_visible,
    } = state;

    let total_lines = buffer.len_lines();
    let metrics = measure_text_editor(ui, total_lines, editor_config);
    let scroll_output = render_rows(
        ui,
        buffer,
        highlight_snapshot,
        editor_config,
        carets,
        selection_ranges,
        active_caret_index,
        caret_visible,
        &metrics,
        events,
    );

    let active_line = carets
        .get(active_caret_index)
        .map_or(1, |(line, _)| *line);
    paint_gutter(
        ui,
        scroll_output.inner_rect,
        &scroll_output.visible_rows,
        active_line,
        &metrics,
    );
}