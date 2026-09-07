use super::text_editor::{TextEditor, TextEditorState};
use crate::chrome::ui::welcome::WelcomeView;
use crate::chrome::ui::{
    FileBinaryInput, FileBinaryView, FileMissingInput, FileMissingView, WorkspaceStartInput,
    WorkspaceStartView,
};
use crate::component::Component;
use crate::documents::DocumentTab;
use crate::documents::tabs::{DocumentTabsInput, DocumentTabsView};
use crate::events::CustomEvent;
use crate::highlighting::HighlightSnapshot;
use crate::highlighting::snapshot_color;
use crate::settings::Config;
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
    pub sidebar_visible: bool,
    pub document_dirty: bool,
    pub document_status: Option<&'a str>,
    pub restoring_session: bool,
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
            sidebar_visible,
            document_dirty,
            document_status,
            restoring_session,
        } = state;
        let mut events = Vec::new();
        if workspace_path.is_none() && buffer.path().is_none() {
            let mut welcome_view = WelcomeView;
            events.extend(welcome_view.render(ui, editor_config));
            return events;
        }

        let nothing_open = buffer.path().is_none()
            && !document_dirty
            && buffer.text().len_chars() == 0
            && !restoring_session;
        if let (Some(path), true) = (workspace_path, nothing_open) {
            let mut start_view = WorkspaceStartView;
            events.extend(start_view.render(
                ui,
                WorkspaceStartInput {
                    workspace_path: path,
                    sidebar_visible,
                },
            ));
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
                        active_path: buffer.path().map(|path| path.as_path()),
                        background: editor_bg,
                    },
                )
                .into_iter()
                .map(CustomEvent::Document),
        );

        // A vanished file is not editable, so the page replaces the text area
        // and leaves the strip reachable for the other tabs.
        let missing = document_tabs
            .get(active_document_index)
            .is_some_and(|tab| tab.missing);
        if missing {
            if let Some(path) = buffer.path() {
                let mut missing_view = FileMissingView;
                missing_view.render(
                    ui,
                    FileMissingInput {
                        path,
                        loader_error: document_status,
                    },
                );
            }
            return events;
        }

        // A binary file is not editable, so a guide page replaces the text
        // area; the tab is a real file though, never struck through.
        let binary = document_tabs
            .get(active_document_index)
            .is_some_and(|tab| tab.binary);
        if binary {
            if let Some(path) = buffer.path() {
                let mut binary_view = FileBinaryView;
                binary_view.render(ui, FileBinaryInput { path });
            }
            return events;
        }

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
