mod gutter;
mod metrics;
mod row;

use crate::component::Component;
use crate::document::DocumentBuffer;
use crate::events::EditingEvent;
use crate::highlighting::HighlightSnapshot;
use crate::settings::Config;
use eframe::egui;
use std::ops::Range;

use self::gutter::{Gutter, GutterInput};
use self::metrics::measure_text_editor;
use self::row::{Rows, RowsInput};

pub struct TextEditorState<'a> {
    pub buffer: &'a DocumentBuffer,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    /// All cursor positions as 1-based (line, column).
    pub carets: &'a [(usize, usize)],
    pub selection_ranges: &'a [Range<usize>],
    pub active_caret_index: usize,
    pub caret_visible: bool,
}

/// Scrollable text area: a gutter box and the virtualized rows sit side by
/// side, each rendered by its own component.
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
        let active_line = carets.get(active_caret_index).map_or(1, |(line, _)| *line);

        // Lay the gutter and rows out as two full-height sibling columns. A
        // `ui.horizontal`/`horizontal_top` sizes its child to interact height
        // and a `ScrollArea` nested inside it does not reliably fill, so scope
        // the rows to an explicit `max_rect` right of the fixed gutter box.
        let available = ui.available_rect_before_wrap();
        let gutter_rect = egui::Rect::from_min_max(
            available.min,
            egui::pos2(
                available.left() + metrics.gutter_total_width,
                available.bottom(),
            ),
        );
        let rows_rect = egui::Rect::from_min_max(
            egui::pos2(gutter_rect.right(), available.top()),
            available.max,
        );

        let mut rows = Rows::default();
        ui.scope_builder(egui::UiBuilder::new().max_rect(rows_rect), |ui| {
            events.extend(rows.render(
                ui,
                RowsInput {
                    buffer,
                    highlight_snapshot,
                    editor_config,
                    carets,
                    selection_ranges,
                    active_caret_index,
                    caret_visible,
                    metrics: &metrics,
                    total_lines,
                },
            ));
        });

        // Clip/fill the gutter to the text viewport so the overflow row the
        // scroll area yields stays hidden even when a horizontal scrollbar
        // shrinks the viewport below the reserved gutter box.
        let gutter_clip = egui::Rect::from_min_max(
            egui::pos2(gutter_rect.left(), rows.inner_rect().top()),
            egui::pos2(gutter_rect.right(), rows.inner_rect().bottom()),
        );
        let mut gutter = Gutter;
        let _ = gutter.render(
            ui,
            GutterInput {
                rect: gutter_clip,
                visible_rows: rows.visible_rows(),
                active_line,
                metrics: &metrics,
            },
        );

        events
    }
}
