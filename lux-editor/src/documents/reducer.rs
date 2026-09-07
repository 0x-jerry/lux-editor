//! Document reducer: IO round-trips, tab lifecycle and pointer-caret edits.

use crate::app::App;
use crate::events::{DocumentEvent, EditingEvent};
use eframe::egui;
use std::path::PathBuf;

impl App {
    /// Document lifecycle & content pipeline: IO round-trips and tabs.
    pub(crate) fn handle_document_event(&mut self, event: DocumentEvent, ctx: &egui::Context) {
        match event {
            DocumentEvent::FilesLoaded {
                entries,
                activate,
                workspace,
            } => {
                self.documents.pending_loads = self.documents.pending_loads.saturating_sub(1);
                // A batch from a workspace the user left would append foreign
                // tabs, steal focus from the new restore and churn the recents.
                if workspace.as_deref() != self.workspace.path.as_deref() || entries.is_empty() {
                    return;
                }
                self.on_files_loaded(entries, activate, ctx);
            }
            DocumentEvent::FileSaved {
                path,
                generation,
                ok,
            } => {
                let active_document = self.active_document_mut();
                if ok {
                    if active_document.edit_generation == generation {
                        active_document.document_dirty = false;
                    }
                    active_document.document_status = Some(format!("Saved {}", path.display()));
                } else {
                    active_document.document_status = Some("Failed to save file".to_string());
                }
                self.update_window_title(ctx);
                self.track_file_open(&path);
                self.on_file_change();
            }
            DocumentEvent::FormattingFinished {
                generation,
                from_save,
                result,
            } => self.on_formatting_finished(generation, from_save, result, ctx),
            DocumentEvent::SwitchDocument(index) => self.switch_to_document(index, ctx),
            DocumentEvent::CloseDocument(index) => self.close_document(index, ctx),
            DocumentEvent::SaveFile => {
                self.save_current_buffer(ctx);
            }
            DocumentEvent::FormatFile => self.format_active_document(ctx),
        }
    }

    /// Text editing & caret: pointer interaction.
    pub(crate) fn handle_editing_event(&mut self, event: EditingEvent) {
        match event {
            EditingEvent::SetCaretFromPointer {
                line_index,
                column,
                selecting,
                add_cursor,
            } => self.set_caret_from_pointer(line_index, column, selecting, add_cursor),
            EditingEvent::SelectWordFromPointer { line_index, column } => {
                self.select_word_from_pointer(line_index, column)
            }
        }
    }

    fn on_files_loaded(
        &mut self,
        entries: Vec<(PathBuf, Result<lux_core::Buffer, String>)>,
        activate: Option<PathBuf>,
        ctx: &egui::Context,
    ) {
        crate::app::startup::stage_once!("first document loaded");
        for (path, _) in &entries {
            self.track_file_open(path);
        }
        self.documents.apply_loaded(entries, activate);
        self.documents.touch_caret_blink();
        self.update_window_title(ctx);
        self.refresh_language_intelligence();
    }
}
