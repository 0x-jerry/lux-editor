mod pointer;
mod text_area;

use crate::config::Config;
use crate::events::CustomEvent;
use crate::language::HighlightSnapshot;
use crate::ui::types::DocumentTab;
use crate::ui::welcome;
use lux_core::Buffer;
use std::path::PathBuf;

pub struct EditorViewState<'a> {
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a Buffer,
    pub document_tabs: &'a [DocumentTab],
    pub active_document_index: usize,
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
        document_tabs,
        active_document_index,
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

    egui::ScrollArea::horizontal()
        .id_salt("document_tabs_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (index, tab) in document_tabs.iter().enumerate() {
                    let selected = index == active_document_index;
                    let label = if selected {
                        format!("[{}]", tab.title)
                    } else {
                        tab.title.clone()
                    };
                    if ui.selectable_label(selected, label).clicked() {
                        events.push(CustomEvent::SwitchDocument(index));
                    }
                    if ui.small_button("x").clicked() {
                        events.push(CustomEvent::CloseDocument(index));
                    }
                }
            });
        });
    ui.separator();

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
