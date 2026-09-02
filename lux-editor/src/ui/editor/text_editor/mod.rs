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
    pub caret_line: usize,
    pub caret_column: usize,
    pub selection_range: Option<Range<usize>>,
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
        caret_line,
        caret_column,
        selection_range,
        caret_visible,
    } = state;

    let total_lines = buffer.len_lines();
    let metrics = measure_text_editor(ui, total_lines, editor_config);
    let scroll_output = render_rows(
        ui,
        buffer,
        highlight_snapshot,
        editor_config,
        caret_line,
        caret_column,
        selection_range,
        caret_visible,
        &metrics,
        events,
    );

    paint_gutter(
        ui,
        scroll_output.inner_rect,
        &scroll_output.visible_rows,
        caret_line,
        &metrics,
    );
}
