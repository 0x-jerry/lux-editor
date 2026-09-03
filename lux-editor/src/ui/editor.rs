mod text_editor;

use crate::config::Config;
use crate::events::CustomEvent;
use crate::language::HighlightSnapshot;
use crate::ui::types::DocumentTab;
use crate::ui::welcome;
use lux_core::Buffer;
use eframe::egui;
use std::ops::Range;
use std::path::PathBuf;

pub struct EditorViewState<'a> {
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a Buffer,
    pub document_tabs: &'a [DocumentTab],
    pub active_document_index: usize,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    /// All cursor positions as 1-based (line, column).
    pub carets: &'a [(usize, usize)],
    pub selection_ranges: &'a [Range<usize>],
    pub active_caret_index: usize,
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
        carets,
        selection_ranges,
        active_caret_index,
        caret_visible,
    } = state;
    if workspace_path.is_none() && buffer.path().is_none() {
        welcome::render_welcome_view(ui, editor_config, events);
        return;
    }

    egui::Frame::new()
        .fill(ui.visuals().code_bg_color)
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            egui::ScrollArea::horizontal()
                .id_salt("document_tabs_scroll")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
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
        });
    ui.add(egui::Separator::default().spacing(0.0));

    text_editor::render_text_editor(
        ui,
        text_editor::TextEditorState {
            buffer,
            highlight_snapshot,
            editor_config,
            carets,
            selection_ranges,
            active_caret_index,
            caret_visible,
        },
        events,
    );
}