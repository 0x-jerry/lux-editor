mod gutter;
mod metrics;
mod row;

use crate::config::Config;
use crate::events::EditingEvent;
use crate::language::HighlightSnapshot;
use crate::ui::component::Component;
use eframe::egui;
use lux_core::Buffer;
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

/// Scrollable text area: rows, gutter and selection painting. The hot path;
/// row/metrics/gutter helpers stay free functions and are untouched.
pub struct TextEditor;

impl Component for TextEditor {
    type Message = EditingEvent;
    type Input<'a> = TextEditorState<'a>;

    fn render(&mut self, ui: &mut egui::Ui, state: Self::Input<'_>) -> Vec<EditingEvent> {
        let TextEditorState {
            buffer,
            highlight_snapshot,
            editor_config,
            carets,
            selection_ranges,
            active_caret_index,
            caret_visible,
        } = state;
        let mut events = Vec::new();

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
            &mut events,
        );

        let active_line = carets.get(active_caret_index).map_or(1, |(line, _)| *line);
        paint_gutter(
            ui,
            scroll_output.inner_rect,
            &scroll_output.visible_rows,
            active_line,
            &metrics,
        );

        events
    }
}
