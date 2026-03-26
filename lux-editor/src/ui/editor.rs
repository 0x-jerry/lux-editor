mod pointer;
mod text_area;

use crate::config::Config;
use crate::events::CustomEvent;
use crate::language::HighlightSnapshot;
use crate::ui::welcome;
use lux_core::Buffer;
use std::path::PathBuf;

pub struct EditorViewState<'a> {
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a Buffer,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    pub caret_line: usize,
    pub caret_column: usize,
    pub selection_len: usize,
    pub caret_visible: bool,
}

pub fn render_editor_view(
    ui: &mut egui::Ui,
    state: EditorViewState<'_>,
    events: &mut Vec<CustomEvent>,
) {
    let EditorViewState {
        workspace_path,
        buffer,
        highlight_snapshot,
        editor_config,
        caret_line,
        caret_column,
        selection_len,
        caret_visible,
    } = state;
    if workspace_path.is_none() && buffer.path().is_none() {
        welcome::render_welcome_view(ui, editor_config, events);
        return;
    }

    text_area::render_text_area(
        ui,
        text_area::TextAreaState {
            buffer,
            highlight_snapshot,
            editor_config,
            caret_line,
            caret_column,
            selection_len,
            caret_visible,
        },
        events,
    );
}
