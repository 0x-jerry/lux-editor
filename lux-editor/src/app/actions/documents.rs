//! Document actions: opening/loading/saving/formatting files, tab lifecycle
//! and the window title. Pure tab-list transitions live on `Documents`; these
//! `impl Ctx` methods add the cross-domain effects (disk reads on the runtime,
//! workspace session tagging, syntax refresh, window chrome).

use crate::app::Ctx;
use crate::document::OpenDocument;
use crate::documents::{openable_tab, run_formatter, tab_with_path};
use crate::events::{DocumentEvent, LoadResult};
use eframe::egui;
use std::path::{Path, PathBuf};

impl Ctx<'_> {
    /// Document lifecycle & content pipeline: IO round-trips and tabs.
    pub(crate) fn handle_document_event(&mut self, event: DocumentEvent) {
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
                self.on_files_loaded(entries, activate);
            }
            DocumentEvent::FileSaved {
                path,
                generation,
                ok,
            } => {
                // The tab the save was started from, not whoever is active now:
                // the user may have switched while the write was in flight.
                if let Some(index) = tab_with_path(&self.documents.tabs, &path) {
                    let document = &mut self.documents.tabs[index];
                    if ok {
                        if document.edit_generation == generation {
                            document.document_dirty = false;
                        }
                        // Baseline against the bytes the save just wrote so the
                        // watcher's own-save event skips the byte compare, and
                        // re-anchor the dirty reference at the saved content.
                        document.record_disk_stat();
                        document.saved_text = document.buffer.text().clone();
                        document.document_status = Some(format!("Saved {}", path.display()));
                    } else {
                        document.document_status = Some("Failed to save file".to_string());
                    }
                }
                self.update_window_title();
                self.track_file_open(&path);
                self.on_file_change();
            }
            DocumentEvent::FilesReconciled { results } => {
                let reload = self.documents.apply_reconcile_results(results);
                self.update_window_title();
                if !reload.is_empty() {
                    // External rewrite of a clean tab: replace it in place.
                    self.load_files(reload, None);
                }
            }
            DocumentEvent::FormattingFinished {
                generation,
                from_save,
                result,
            } => self.on_formatting_finished(generation, from_save, result),
            DocumentEvent::SwitchDocument(index) => self.switch_to_document(index),
            DocumentEvent::CloseDocument(index) => self.close_document(index),
            DocumentEvent::SaveFile => {
                self.save_current_buffer();
            }
            DocumentEvent::FormatFile => self.format_active_document(),
        }
    }

    pub(crate) fn open_file(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        if let Some(index) = openable_tab(&self.documents.tabs, &path) {
            self.switch_to_document(index);
            return;
        }
        self.load_files(vec![path.clone()], Some(path));
    }

    /// Restore files as tabs from one task reading them in order, so the strip
    /// and the focused tab come back as saved rather than in disk order.
    pub(crate) fn open_files(&mut self, paths: Vec<PathBuf>, activate: Option<PathBuf>) {
        let mut paths = paths
            .into_iter()
            .map(|path| path.canonicalize().unwrap_or(path))
            .collect::<Vec<_>>();
        paths.retain(|path| openable_tab(&self.documents.tabs, path).is_none());
        if paths.is_empty() {
            // Nothing left to read, but the remembered tab still wants focus.
            if let Some(path) = activate
                && let Some(index) = tab_with_path(&self.documents.tabs, &path)
            {
                self.switch_to_document(index);
            }
            return;
        }
        self.load_files(paths, activate);
    }

    /// Move tabs to `new` when the tree renamed `old` out from under them.
    pub(crate) fn on_path_renamed(&mut self, old: &Path, new: &Path) {
        let mut moved = false;
        for document in self.documents.tabs.iter_mut() {
            if document.buffer.path().is_some_and(|path| path == old) {
                document.buffer.set_path(new);
                document.missing = false;
                moved = true;
            }
        }
        if moved {
            self.refresh_language_intelligence();
        }
    }

    /// Read `paths` into tabs; `activate` names the one to focus, `None` keeps
    /// the current one. Each call is one batch, so a restore keeps its order.
    pub(crate) fn load_files(&mut self, paths: Vec<PathBuf>, activate: Option<PathBuf>) {
        self.documents.pending_loads += 1;
        let workspace = self.workspace.path.clone();
        let event_tx = self.runtime.event_tx.clone();
        let wake = self.egui_ctx().clone();
        self.runtime.spawn(async move {
            let mut entries = Vec::with_capacity(paths.len());
            for path in paths {
                let result = match tokio::fs::read(&path).await {
                    Err(err) => LoadResult::Missing(err.to_string()),
                    Ok(bytes) => match std::str::from_utf8(&bytes) {
                        Ok(text) => {
                            let mut buffer = crate::document::DocumentBuffer::new();
                            buffer.set_path(&path);
                            buffer.insert(0, text);
                            LoadResult::Loaded(buffer)
                        }
                        // Not valid UTF-8: a binary file, not a vanished one.
                        Err(_) => LoadResult::Binary,
                    },
                };
                entries.push((path, result));
            }
            let _ = event_tx.send(crate::events::CustomEvent::Document(
                DocumentEvent::FilesLoaded {
                    entries,
                    activate,
                    workspace,
                },
            ));
            wake.request_repaint();
        });
    }

    pub(crate) fn save_current_buffer(&mut self) -> bool {
        if self.documents.active_document().missing {
            self.documents.active_document_mut().document_status =
                Some("File not found — nothing to save".to_string());
            return false;
        }
        if self.documents.active_document().binary {
            self.documents.active_document_mut().document_status =
                Some("Binary file — not editable".to_string());
            return false;
        }
        let active_path = self.documents.active_document().buffer.path().cloned();
        if active_path.is_none() {
            if let Some(path) = rfd::FileDialog::new().save_file() {
                self.documents.buffer_mut().set_path(&path);
            } else {
                self.documents.active_document_mut().document_status =
                    Some("Save cancelled".to_string());
                return false;
            }
        }

        let save_path = self.documents.buffer().path().cloned().unwrap();
        let text = self.documents.buffer().text().to_string();
        let generation = self.documents.active_document().edit_generation;
        let formatter = self.settings.editor_config.settings.formatter.clone();
        let format_on_save = formatter.format_on_save && !formatter.command.trim().is_empty();
        let event_tx = self.runtime.event_tx.clone();
        let wake = self.egui_ctx().clone();
        self.runtime.spawn_blocking(move || {
            let mut to_write = text.clone();
            let mut formatted_result: Option<Result<String, String>> = None;
            if format_on_save {
                match run_formatter(&formatter.command, &formatter.args, &text) {
                    Ok(formatted) if formatted != text => {
                        to_write = formatted.clone();
                        formatted_result = Some(Ok(formatted));
                    }
                    Ok(_) => {}
                    Err(err) => formatted_result = Some(Err(err)),
                }
            }
            let ok = std::fs::write(&save_path, to_write).is_ok();
            let _ = event_tx.send(crate::events::CustomEvent::Document(
                DocumentEvent::FileSaved {
                    path: save_path,
                    generation,
                    ok,
                },
            ));
            if let Some(result) = formatted_result {
                let _ = event_tx.send(crate::events::CustomEvent::Document(
                    DocumentEvent::FormattingFinished {
                        generation,
                        from_save: true,
                        result,
                    },
                ));
            }
            wake.request_repaint();
        });
        true
    }

    /// Reflect the active document in the window title.
    pub(crate) fn update_window_title(&mut self) {
        let title = self.documents.window_title();
        self.egui_ctx()
            .send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    pub(crate) fn switch_to_document(&mut self, index: usize) {
        if index >= self.documents.tabs.len() {
            return;
        }
        self.documents.active_document = index;
        self.documents.touch_caret_blink();
        self.update_window_title();
        self.refresh_language_intelligence();
    }

    pub(crate) fn close_document(&mut self, index: usize) {
        if index >= self.documents.tabs.len() {
            return;
        }

        if self.documents.tabs[index].document_dirty {
            self.documents.tabs[index].document_status =
                Some("Unsaved changes — save before closing".to_string());
            return;
        }

        if self.documents.tabs.len() == 1 {
            self.documents.tabs[0] = OpenDocument::new_empty();
            self.documents.active_document = 0;
            self.documents.touch_caret_blink();
            self.update_window_title();
            self.refresh_language_intelligence();
            return;
        }

        self.documents.tabs.remove(index);
        if self.documents.active_document >= self.documents.tabs.len() {
            self.documents.active_document = self.documents.tabs.len().saturating_sub(1);
        } else if index < self.documents.active_document {
            self.documents.active_document = self.documents.active_document.saturating_sub(1);
        }
        self.documents.touch_caret_blink();
        self.update_window_title();
        self.refresh_language_intelligence();
    }

    fn on_files_loaded(&mut self, entries: Vec<(PathBuf, LoadResult)>, activate: Option<PathBuf>) {
        crate::app::startup::stage_once!("first document loaded");
        for (path, _) in &entries {
            self.track_file_open(path);
        }
        self.documents.apply_loaded(entries, activate);
        self.documents.touch_caret_blink();
        self.update_window_title();
        self.refresh_language_intelligence();
    }

    /// Format the active document by piping it through the configured external
    /// formatter. The result is applied only if the buffer generation still
    /// matches (i.e. the user has not typed since the request was launched).
    pub(crate) fn format_active_document(&mut self) {
        // The command palette reaches this bypassing the keyboard gate; without
        // the guard a formatter on the empty buffer of a missing/binary tab
        // would mark it dirty — a tab that can then neither close nor save.
        if self.documents.active_document().missing || self.documents.active_document().binary {
            self.documents.active_document_mut().document_status =
                Some("Nothing to format here".to_string());
            return;
        }
        let formatter = self.settings.editor_config.settings.formatter.clone();
        if formatter.command.trim().is_empty() {
            self.documents.active_document_mut().document_status =
                Some("No formatter configured".to_string());
            return;
        }

        let text = self.documents.active_document().buffer.text().to_string();
        let generation = self.documents.active_document().edit_generation;
        let event_tx = self.runtime.event_tx.clone();
        let wake = self.egui_ctx().clone();
        self.documents.active_document_mut().document_status = Some("Formatting…".to_string());

        self.runtime.spawn_blocking(move || {
            let result = run_formatter(&formatter.command, &formatter.args, &text);
            let _ = event_tx.send(crate::events::CustomEvent::Document(
                DocumentEvent::FormattingFinished {
                    generation,
                    from_save: false,
                    result,
                },
            ));
            wake.request_repaint();
        });
    }

    pub(crate) fn on_formatting_finished(
        &mut self,
        generation: u64,
        from_save: bool,
        result: Result<String, String>,
    ) {
        // The buffer moved on while the formatter was running; the result is
        // stale and must not clobber newer edits.
        if self.documents.active_document().edit_generation != generation {
            return;
        }

        match result {
            Ok(formatted) => {
                if formatted
                    .chars()
                    .eq(self.documents.active_document().buffer.text().chars())
                {
                    self.documents.active_document_mut().document_status =
                        Some("Already formatted".to_string());
                    return;
                }
                let total_chars = self.documents.active_document().buffer.text().len_chars();
                let positions = self
                    .documents
                    .active_document()
                    .caret_state
                    .caret_chars_snapshot();
                if self.documents.apply_edit(0, total_chars, &formatted) {
                    let active_document = self.documents.active_document_mut();
                    let len = active_document.buffer.text().len_chars();
                    let clamped = positions
                        .iter()
                        .map(|pos| (*pos).min(len))
                        .collect::<Vec<usize>>();
                    active_document
                        .caret_state
                        .set_all_caret_chars(&clamped, &active_document.buffer);
                    if from_save {
                        // The save task already wrote this text to disk, so the
                        // buffer is in the saved state, not dirty.
                        active_document.document_dirty = false;
                        active_document.saved_text = active_document.buffer.text().clone();
                        active_document.document_status = Some("Formatted".to_string());
                    } else {
                        active_document.document_status = Some("Formatted".to_string());
                    }
                    self.update_window_title();
                    self.schedule_language_refresh();
                }
            }
            Err(err) => {
                self.documents.active_document_mut().document_status =
                    Some(format!("Format failed: {}", err));
            }
        }
    }
}
