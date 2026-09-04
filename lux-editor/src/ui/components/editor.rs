use super::document_tabs::{DocumentTabsInput, DocumentTabsView};
use super::text_editor::{TextEditor, TextEditorState};
use super::welcome::WelcomeView;
use crate::config::Config;
use crate::events::CustomEvent;
use crate::language::HighlightSnapshot;
use crate::ui::component::Component;
use crate::ui::highlight::snapshot_color;
use crate::ui::types::DocumentTab;
use eframe::egui;
use lux_core::Buffer;
use std::ops::Range;
use std::path::PathBuf;

/// The editor tab strip, welcome fallback and text area.
pub struct EditorView;

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

impl Component for EditorView {
    type Message = CustomEvent;
    type Input<'a> = EditorViewState<'a>;

    fn render(&mut self, ui: &mut egui::Ui, state: Self::Input<'_>) -> Vec<CustomEvent> {
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
        let mut events = Vec::new();
        if workspace_path.is_none() && buffer.path().is_none() {
            let mut welcome_view = WelcomeView;
            events.extend(welcome_view.render(ui, editor_config));
            return events;
        }

        let editor_bg = snapshot_color(highlight_snapshot.background, ui.visuals().code_bg_color);

        let mut tabs_view = DocumentTabsView;
        events.extend(
            tabs_view
                .render(
                    ui,
                    DocumentTabsInput {
                        tabs: document_tabs,
                        active_index: active_document_index,
                        background: editor_bg,
                    },
                )
                .into_iter()
                .map(CustomEvent::Document),
        );

        let mut text_editor = TextEditor;
        events.extend(
            text_editor
                .render(
                    ui,
                    TextEditorState {
                        buffer,
                        highlight_snapshot,
                        editor_config,
                        carets,
                        selection_ranges,
                        active_caret_index,
                        caret_visible,
                    },
                )
                .into_iter()
                .map(CustomEvent::Editing),
        );
        events
    }
}
