use super::metrics::TextEditorMetrics;
use crate::component::Component;
use crate::document::DocumentBuffer;
use crate::document::ui::{ScrollPane, ScrollSync};
use crate::events::EditingEvent;
use crate::highlighting::HighlightSnapshot;
use crate::highlighting::build_highlighted_line_job;
use crate::settings::Config;
use eframe::egui;
use std::ops::Range;

pub struct VisibleRow {
    pub index: usize,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Clone, Default)]
struct RevealState {
    last_caret_line: usize,
    offset: f32,
    target_offset: Option<f32>,
}

pub struct RowsInput<'a> {
    pub buffer: &'a DocumentBuffer,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    pub carets: &'a [(usize, usize)],
    pub selection_ranges: &'a [Range<usize>],
    pub active_caret_index: usize,
    pub caret_visible: bool,
    pub metrics: &'a TextEditorMetrics,
    pub total_lines: usize,
    /// Scroll mirroring with the markdown preview; `None` when it is closed.
    pub scroll_sync: Option<&'a mut ScrollSync>,
}

/// The scrollable text area: virtualizes rows via a scroll area and tracks the
/// scroll/reveal state. Rebuilds `visible_rows` and `inner_rect` each frame for
/// the gutter to consume.
pub struct Rows {
    visible_rows: Vec<VisibleRow>,
    inner_rect: egui::Rect,
}

impl Default for Rows {
    fn default() -> Self {
        Rows {
            visible_rows: Vec::new(),
            inner_rect: egui::Rect::ZERO,
        }
    }
}

impl Rows {
    pub fn visible_rows(&self) -> &[VisibleRow] {
        &self.visible_rows
    }

    pub fn inner_rect(&self) -> egui::Rect {
        self.inner_rect
    }
}

impl Component for Rows {
    type Message = EditingEvent;
    type Input<'a> = RowsInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<EditingEvent> {
        let mut events = Vec::new();
        // show_rows virtualizes on `row_height + item_spacing.y`; a contiguous
        // grid of lines (no gaps) is what a code editor wants, so zero the spacing
        // before the scroll area reads it.
        ui.spacing_mut().item_spacing.y = 0.0;

        let (active_line, _) = input
            .carets
            .get(input.active_caret_index)
            .copied()
            .unwrap_or((1, 1));
        // Scroll and reveal state are keyed per document: a first-time open starts
        // at the top, and switching tabs restores each file's own position.
        let document_salt = input
            .buffer
            .path()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled".to_owned());
        let reveal_id = egui::Id::new(("editor_text_editor_reveal", document_salt.clone()));
        let mut reveal: RevealState =
            ui.data_mut(|data| data.get_temp(reveal_id).unwrap_or_default());

        // Don't fight the user dragging the scrollbar: only reveal on caret moves.
        let user_scrolling = ui.input(|input| {
            input.smooth_scroll_delta.y.abs() > 0.0
                || input
                    .raw
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::MouseWheel { .. }))
        });
        if user_scrolling {
            reveal.last_caret_line = active_line;
        }
        let viewport_height = ui.available_height();
        let caret_row_top = active_line.saturating_sub(1) as f32 * input.metrics.row_height;
        if !user_scrolling && reveal.last_caret_line != active_line {
            reveal.last_caret_line = active_line;
            let margin = input.metrics.row_height * 3.0;
            let content_height = input.total_lines as f32 * input.metrics.row_height;
            let max_scroll = (content_height - viewport_height).max(0.0);
            let caret_bottom = caret_row_top + input.metrics.row_height;
            if caret_row_top < reveal.offset || caret_bottom > reveal.offset + viewport_height {
                reveal.target_offset = Some((caret_row_top - margin).clamp(0.0, max_scroll));
            } else {
                reveal.target_offset = None;
            }
        }

        let mut scroll_area = egui::ScrollArea::both()
            .id_salt(("editor_text_editor_scroll", document_salt))
            .scroll_source(egui::scroll_area::ScrollSource::MOUSE_WHEEL)
            .auto_shrink([false, false]);
        // Revealing the caret outranks mirroring the preview.
        let follow_offset = if reveal.target_offset.is_some() {
            None
        } else {
            input
                .scroll_sync
                .as_ref()
                .and_then(|sync| sync.follow_offset(ScrollPane::Editor))
        };
        if let Some(target) = reveal.target_offset {
            scroll_area = scroll_area.vertical_scroll_offset(target);
        } else if let Some(offset) = follow_offset {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        }

        self.visible_rows.clear();
        let scroll_output = scroll_area.show_rows(
            ui,
            input.metrics.row_height,
            input.total_lines,
            |ui, row_range| {
                for line_index in row_range {
                    let mut row = Row;
                    events.extend(row.render(
                        ui,
                        RowInput {
                            line_index,
                            buffer: input.buffer,
                            highlight_snapshot: input.highlight_snapshot,
                            editor_config: input.editor_config,
                            carets: input.carets,
                            selection_ranges: input.selection_ranges,
                            active_caret_index: input.active_caret_index,
                            caret_visible: input.caret_visible,
                            metrics: input.metrics,
                            visible_rows: &mut self.visible_rows,
                        },
                    ));
                }
            },
        );

        reveal.offset = scroll_output.state.offset.y;
        if let Some(sync) = input.scroll_sync {
            if reveal.target_offset.is_some() {
                sync.claim(ScrollPane::Editor);
            }
            sync.report(
                ScrollPane::Editor,
                reveal.offset,
                scroll_output.inner_rect,
                scroll_output.content_size,
            );
        }
        reveal.target_offset = None;
        ui.data_mut(|data| data.insert_temp(reveal_id, reveal));

        self.inner_rect = scroll_output.inner_rect;

        events
    }
}

pub struct RowInput<'a> {
    pub line_index: usize,
    pub buffer: &'a DocumentBuffer,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    pub carets: &'a [(usize, usize)],
    pub selection_ranges: &'a [Range<usize>],
    pub active_caret_index: usize,
    pub caret_visible: bool,
    pub metrics: &'a TextEditorMetrics,
    pub visible_rows: &'a mut Vec<VisibleRow>,
}

/// A single visible line: highlights, selection, caret and pointer interaction.
pub struct Row;

impl Component for Row {
    type Message = EditingEvent;
    type Input<'a> = RowInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<EditingEvent> {
        let line_start = input.buffer.text().line_to_char(input.line_index);
        let line_text_owned = input
            .buffer
            .line(input.line_index)
            .and_then(|mut lines| lines.next())
            .map(|line| line.to_string())
            .unwrap_or_default();
        let line_text = display_line_text(&line_text_owned);
        let line_len = line_text.chars().count();

        let tokens = input.highlight_snapshot.line_tokens.get(input.line_index);
        let default_color = crate::highlighting::snapshot_color(
            input.highlight_snapshot.foreground,
            ui.visuals().text_color(),
        );
        let job = build_highlighted_line_job(
            line_text,
            tokens.map(Vec::as_slice).unwrap_or(&[]),
            input.editor_config.settings.font.size,
            default_color,
        );
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
        let row_width = galley.size().x.max(ui.available_width());
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(row_width, input.metrics.row_height),
            egui::Sense::click_and_drag(),
        );
        let response = response.on_hover_and_drag_cursor(egui::CursorIcon::Text);
        let text_origin = egui::pos2(rect.left(), rect.top());

        let mut events = Vec::new();
        for range in input.selection_ranges {
            paint_selection(
                ui,
                &input,
                line_start,
                line_len,
                range,
                &galley,
                text_origin,
            );
        }
        ui.painter()
            .galley(text_origin, galley.clone(), ui.visuals().text_color());
        events.extend(push_pointer_events(ui, &input, &galley, &rect, &response));
        for (index, (caret_line, caret_column)) in input.carets.iter().enumerate() {
            if *caret_line != input.line_index + 1 {
                continue;
            }
            let char_pos = line_start + caret_column.saturating_sub(1);
            let covered = input
                .selection_ranges
                .iter()
                .any(|range| char_pos >= range.start && char_pos < range.end);
            if covered {
                continue;
            }
            let show = input.caret_visible || index != input.active_caret_index;
            paint_caret(ui, &galley, text_origin, *caret_column, show, input.metrics);
        }

        input.visible_rows.push(VisibleRow {
            index: input.line_index,
            top: rect.top(),
            bottom: rect.bottom(),
        });
        events
    }
}

fn display_line_text(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn paint_selection(
    ui: &egui::Ui,
    input: &RowInput<'_>,
    line_start: usize,
    line_len: usize,
    range: &Range<usize>,
    galley: &egui::text::Galley,
    text_origin: egui::Pos2,
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
        text_origin.x
            + galley
                .pos_from_cursor(egui::text::CCursor::new(covered_start - line_start))
                .min
                .x
    };
    let x1 = if range.end >= line_end {
        text_origin.x + galley.size().x
    } else {
        text_origin.x
            + galley
                .pos_from_cursor(egui::text::CCursor::new(covered_end - line_start))
                .min
                .x
    };
    let rect = egui::Rect::from_min_max(
        egui::pos2(x0, text_origin.y),
        egui::pos2(x1, text_origin.y + input.metrics.row_height),
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
    let x = text_origin.x
        + galley
            .pos_from_cursor(egui::text::CCursor::new(column))
            .min
            .x;
    ui.painter().line_segment(
        [
            egui::pos2(x, text_origin.y),
            egui::pos2(x, text_origin.y + metrics.row_height),
        ],
        egui::Stroke::new(1.5, ui.visuals().text_color()),
    );
}

fn push_pointer_events(
    ui: &egui::Ui,
    input: &RowInput<'_>,
    galley: &egui::text::Galley,
    rect: &egui::Rect,
    response: &egui::Response,
) -> Vec<EditingEvent> {
    let line_index = input.line_index;
    let text_origin = egui::pos2(rect.left(), rect.top());
    let mut events = Vec::new();
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
        events.push(EditingEvent::SelectWordFromPointer {
            line_index,
            column: column_from(pointer).0,
        });
    } else if response.clicked_by(egui::PointerButton::Primary)
        && let Some(pointer) = response.interact_pointer_pos()
    {
        // egui also reports a double-click as a click; the caret placement
        // would collapse the word selection the branch above just made.
        let selecting = ui.input(|input| input.modifiers.shift);
        let add_cursor = ui.input(|input| input.modifiers.command || input.modifiers.ctrl);
        events.push(EditingEvent::SetCaretFromPointer {
            line_index,
            column: column_from(pointer).0,
            selecting,
            add_cursor,
        });
    }

    // Drags keep targeting the row where the press started, so project the
    // pointer onto the row it is over and let the app clamp to real lines.
    let line_column_from = |pointer: egui::Pos2| {
        let row_delta = ((pointer.y - rect.top()) / input.metrics.row_height).floor() as isize;
        let pointed_line = (line_index as isize + row_delta).max(0) as usize;
        let pointed_top = rect.top() + row_delta as f32 * input.metrics.row_height;
        // The pressed row's galley only maps x within its own text, so over a
        // different line its end clamps the cursor to the pressed line's width.
        // Lay the pointed line out the same way to track the caret it paints.
        let cursor_offset = egui::vec2(pointer.x - text_origin.x, pointer.y - pointed_top);
        let column = if pointed_line == line_index {
            galley.cursor_from_pos(cursor_offset).index.0
        } else {
            let line_text_owned = input
                .buffer
                .line(pointed_line)
                .and_then(|mut lines| lines.next())
                .map(|line| line.to_string())
                .unwrap_or_default();
            if line_text_owned.is_empty() {
                galley.cursor_from_pos(cursor_offset).index.0
            } else {
                let tokens = input.highlight_snapshot.line_tokens.get(pointed_line);
                let default_color = crate::highlighting::snapshot_color(
                    input.highlight_snapshot.foreground,
                    ui.visuals().text_color(),
                );
                let job = build_highlighted_line_job(
                    display_line_text(&line_text_owned),
                    tokens.map(Vec::as_slice).unwrap_or(&[]),
                    input.editor_config.settings.font.size,
                    default_color,
                );
                ui.ctx()
                    .fonts_mut(|fonts| fonts.layout_job(job))
                    .cursor_from_pos(cursor_offset)
                    .index
                    .0
            }
        };
        (pointed_line, column)
    };

    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(origin) = ui.input(|input| input.pointer.press_origin())
    {
        // Clicked only reports on release-without-drag, so when a drag is
        // decided the caret still sits where the last interaction left it.
        // Re-home it at the press point first; the drag events below then
        // anchor the selection there instead of on the stale caret.
        let (line, column) = line_column_from(origin);
        events.push(EditingEvent::SetCaretFromPointer {
            line_index: line,
            column,
            selecting: false,
            add_cursor: false,
        });
    }

    if response.dragged_by(egui::PointerButton::Primary)
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let (line, column) = line_column_from(pointer);
        events.push(EditingEvent::SetCaretFromPointer {
            line_index: line,
            column,
            selecting: true,
            add_cursor: false,
        });
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::ui::text_editor::metrics::measure_text_editor;

    fn render_row(
        ctx: &egui::Context,
        raw: egui::RawInput,
        buffer: &DocumentBuffer,
        events: &mut Vec<EditingEvent>,
    ) {
        let config = Config::default();
        let snapshot = HighlightSnapshot::default();
        let mut visible_rows = Vec::new();
        // The frame allocates text textures; clear them so the context can drop.
        let mut output = ctx.run_ui(raw, |ui| {
            let metrics = measure_text_editor(ui, 1, &config);
            let mut row = Row;
            events.extend(row.render(
                ui,
                RowInput {
                    line_index: 0,
                    buffer,
                    highlight_snapshot: &snapshot,
                    editor_config: &config,
                    carets: &[(1, 1)],
                    selection_ranges: &[],
                    active_caret_index: 0,
                    caret_visible: true,
                    metrics: &metrics,
                    visible_rows: &mut visible_rows,
                },
            ));
        });
        output.textures_delta.clear();
    }

    fn click(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }

    #[test]
    fn double_click_selects_word_and_skips_the_caret_placement() {
        let ctx = egui::Context::default();
        let mut buffer = DocumentBuffer::new();
        buffer.insert(0, "hello world");
        // The first row starts at the top-left of the viewport; click into it.
        let pos = egui::pos2(40.0, 8.0);
        let empty = || egui::RawInput {
            events: vec![],
            ..Default::default()
        };
        let click_frame = |time: f64| egui::RawInput {
            time: Some(time),
            events: vec![click(pos, true), click(pos, false)],
            ..Default::default()
        };

        // egui hit-tests against the previous pass's widget rects, so a frame
        // with no input must lay the rows out before a click can land.
        let mut warmup = Vec::new();
        render_row(&ctx, empty(), &buffer, &mut warmup);
        assert!(warmup.is_empty());

        // A single click places the caret.
        let mut first = Vec::new();
        render_row(&ctx, click_frame(0.05), &buffer, &mut first);
        assert_eq!(first.len(), 1);
        assert!(matches!(first[0], EditingEvent::SetCaretFromPointer { .. }));

        // A second click inside the double-click window selects the word. The
        // same release also counts as one more click, which would collapse the
        // word selection, so no caret placement may follow it either.
        let mut second = Vec::new();
        render_row(&ctx, click_frame(0.1), &buffer, &mut second);
        assert_eq!(second.len(), 1);
        assert!(matches!(
            second[0],
            EditingEvent::SelectWordFromPointer { line_index: 0, .. }
        ));
    }
}
