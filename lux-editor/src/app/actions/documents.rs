//! Document actions: opening/loading/saving/formatting files, tab lifecycle
//! and the window title. Pure tab-list transitions live on `TabManager`; these
//! `impl Ctx` methods add the cross-domain effects (disk reads on the runtime,
//! workspace session tagging, syntax refresh, window chrome).

use crate::app::Ctx;
use crate::document::run_formatter;
use crate::events::{DocumentEvent, LoadResult};
use crate::tabs::{openable_tab, tab_with_path};
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
                self.tabs.pending_loads = self.tabs.pending_loads.saturating_sub(1);
                // A batch from a workspace the user left would append foreign
                // tabs, steal focus from the new restore and churn the recents.
                if workspace.as_deref() != self.workspace.path.as_deref() || entries.is_empty() {
                    return;
                }
                // Consume a pending configuration restore only for the
                // workspace it was requested for; other batches leave it armed.
                let restore_configuration = self
                    .frame
                    .pending_configuration_restore
                    .as_deref()
                    .is_some_and(|w| Some(w) == self.workspace.path.as_deref());
                self.frame.pending_configuration_restore = None;
                self.on_files_loaded(entries, activate);
                if restore_configuration {
                    self.open_configuration_tab();
                }
            }
            DocumentEvent::FileSaved {
                path,
                generation,
                ok,
            } => {
                // The tab the save was started from, not whoever is active now:
                // the user may have switched while the write was in flight.
                let mut saved = None;
                if let Some(index) = tab_with_path(&self.tabs.tabs, &path) {
                    let id = self.tabs.tabs[index].id;
                    if let Some(document) = self.tabs.tabs[index].content.as_text_mut() {
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
                        saved = Some((id, document.document_dirty));
                    }
                }
                if let Some((id, clean)) = saved {
                    if self.frame.pending_close_after_save == Some(id) {
                        self.frame.pending_close_after_save = None;
                        // Close only once the write landed and no newer edit
                        // slipped in while it was in flight.
                        if ok && clean {
                            self.close_tab(id);
                        }
                    }
                } else if self
                    .frame
                    .pending_close_after_save
                    .is_some_and(|id| !self.tabs.tabs.iter().any(|tab| tab.id == id))
                {
                    self.frame.pending_close_after_save = None;
                }
                self.update_window_title();
                self.track_file_open(&path);
                self.on_file_change();
            }
            DocumentEvent::FilesReconciled { results } => {
                let reload = self.tabs.apply_reconcile_results(results);
                self.update_window_title();
                if !reload.is_empty() {
                    // External rewrite of a clean tab: replace it in place.
                    self.load_files(reload, None);
                }
            }
            DocumentEvent::FormattingFinished {
                path,
                generation,
                from_save,
                result,
            } => self.on_formatting_finished(path, generation, from_save, result),
            DocumentEvent::SwitchTab(id) => self.switch_to_tab(id),
            DocumentEvent::CloseTab(id) => self.request_close_tab(id),
            DocumentEvent::SaveAndCloseTab(id) => self.save_and_close_tab(id),
            DocumentEvent::DiscardTab(id) => self.discard_tab(id),
            DocumentEvent::SaveFile => {
                self.save_current_buffer();
            }
            DocumentEvent::FormatFile => self.format_active_document(),
        }
    }

    pub(crate) fn open_file(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        if let Some(index) = openable_tab(&self.tabs.tabs, &path) {
            self.switch_to_tab(self.tabs.tabs[index].id);
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
        paths.retain(|path| openable_tab(&self.tabs.tabs, path).is_none());
        if paths.is_empty() {
            // Nothing left to read, but the remembered tab still wants focus.
            if let Some(path) = activate
                && let Some(index) = tab_with_path(&self.tabs.tabs, &path)
            {
                self.switch_to_tab(self.tabs.tabs[index].id);
            }
            return;
        }
        self.load_files(paths, activate);
    }

    /// Move tabs to `new` when the tree renamed `old` out from under them.
    pub(crate) fn on_path_renamed(&mut self, old: &Path, new: &Path) {
        let mut moved = false;
        for tab in self.tabs.tabs.iter_mut() {
            let Some(document) = tab.content.as_text_mut() else {
                continue;
            };
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
        self.tabs.pending_loads += 1;
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
        let Some(active_document) = self.tabs.active_text() else {
            return false;
        };
        if active_document.missing {
            self.tabs.active_text_mut().unwrap().document_status =
                Some("File not found — nothing to save".to_string());
            return false;
        }
        if active_document.binary {
            self.tabs.active_text_mut().unwrap().document_status =
                Some("Binary file — not editable".to_string());
            return false;
        }
        let active_path = active_document.buffer.path().cloned();
        if active_path.is_none() {
            if let Some(path) = rfd::FileDialog::new().save_file() {
                self.tabs.active_text_mut().unwrap().buffer.set_path(&path);
            } else {
                self.tabs.active_text_mut().unwrap().document_status =
                    Some("Save cancelled".to_string());
                return false;
            }
        }

        let (save_path, text, generation) = {
            let active_document = self.tabs.active_text().unwrap();
            let save_path = active_document.buffer.path().cloned().unwrap();
            let text = active_document.buffer.text().to_string();
            let generation = active_document.edit_generation;
            (save_path, text, generation)
        };
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
                    path: save_path.clone(),
                    generation,
                    ok,
                },
            ));
            if let Some(result) = formatted_result {
                let _ = event_tx.send(crate::events::CustomEvent::Document(
                    DocumentEvent::FormattingFinished {
                        path: Some(save_path),
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
        let title = self.tabs.window_title();
        self.egui_ctx()
            .send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    pub(crate) fn switch_to_tab(&mut self, id: u64) {
        if !self.tabs.focus_tab(id) {
            return;
        }
        self.tabs.touch_caret_blink();
        self.update_window_title();
        self.refresh_language_intelligence();
    }

    pub(crate) fn close_tab(&mut self, id: u64) {
        let Some(index) = self.tabs.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };

        // A dirty text tab refuses to close; the strip keeps its dirty-dot
        // affordance. Other content (configuration) closes unconditionally.
        if let Some(document) = self.tabs.tabs[index].content.as_text_mut()
            && document.document_dirty
        {
            document.document_status = Some("Unsaved changes — save before closing".to_string());
            return;
        }

        self.remove_tab_at(index);
    }

    /// Close whichever tab is active (the ⌘W / Close Tab target).
    pub(crate) fn close_active_tab(&mut self) {
        let id = self.tabs.active_id();
        self.request_close_tab(id);
    }

    /// Close `id`, prompting for save/discard when it has unsaved changes.
    pub(crate) fn request_close_tab(&mut self, id: u64) {
        if self.tabs.close_needs_confirmation(id) {
            self.chrome.close_prompt.request(id);
            return;
        }
        self.close_tab(id);
    }

    /// Close `id` without saving, discarding any unsaved changes.
    pub(crate) fn discard_tab(&mut self, id: u64) {
        self.chrome.close_prompt.cancel();
        let Some(index) = self.tabs.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        self.remove_tab_at(index);
    }

    /// Save `id` and close it once the write lands clean. A cancelled save-as
    /// or a failed write leaves the tab open and dirty.
    pub(crate) fn save_and_close_tab(&mut self, id: u64) {
        self.chrome.close_prompt.cancel();
        if !self.tabs.focus_tab(id) {
            return;
        }
        self.tabs.touch_caret_blink();
        self.update_window_title();
        self.refresh_language_intelligence();
        if self.save_current_buffer() {
            self.frame.pending_close_after_save = Some(id);
        }
    }

    fn remove_tab_at(&mut self, index: usize) {
        self.tabs.remove_tab(index);
        self.tabs.touch_caret_blink();
        self.update_window_title();
        self.refresh_language_intelligence();
    }

    /// Open (or focus) the configuration tab.
    pub(crate) fn open_configuration_tab(&mut self) {
        self.tabs.open_configuration();
        self.tabs.touch_caret_blink();
        self.update_window_title();
    }

    fn on_files_loaded(&mut self, entries: Vec<(PathBuf, LoadResult)>, activate: Option<PathBuf>) {
        crate::app::startup::stage_once!("first document loaded");
        for (path, _) in &entries {
            self.track_file_open(path);
        }
        // A reloaded image tab must not show the previous bytes: the egui
        // loaders cache by URI forever, so drop the entry whenever the file is
        // re-read (a no-op on first open).
        let ctx = self.egui_ctx();
        for (path, result) in &entries {
            if matches!(result, LoadResult::Binary) && crate::chrome::ui::is_image_path(path) {
                ctx.forget_image(&crate::chrome::ui::file_image::file_uri(path));
            }
        }
        self.tabs.apply_loaded(entries, activate);
        self.tabs.touch_caret_blink();
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
        let Some(active_document) = self.tabs.active_text() else {
            return;
        };
        if active_document.missing || active_document.binary {
            self.tabs.active_text_mut().unwrap().document_status =
                Some("Nothing to format here".to_string());
            return;
        }
        let formatter = self.settings.editor_config.settings.formatter.clone();
        if formatter.command.trim().is_empty() {
            self.tabs.active_text_mut().unwrap().document_status =
                Some("No formatter configured".to_string());
            return;
        }

        let text = active_document.buffer.text().to_string();
        let path = active_document.buffer.path().cloned();
        let generation = active_document.edit_generation;
        let event_tx = self.runtime.event_tx.clone();
        let wake = self.egui_ctx().clone();
        self.tabs.active_text_mut().unwrap().document_status = Some("Formatting…".to_string());

        self.runtime.spawn_blocking(move || {
            let result = run_formatter(&formatter.command, &formatter.args, &text);
            let _ = event_tx.send(crate::events::CustomEvent::Document(
                DocumentEvent::FormattingFinished {
                    path,
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
        path: Option<PathBuf>,
        generation: u64,
        from_save: bool,
        result: Result<String, String>,
    ) {
        // The tab the formatter ran on, not whoever is active now: a
        // close-after-save can drop it before this lands, and applying the
        // result to a different document would corrupt it.
        let index = match path.as_deref() {
            Some(path) => match tab_with_path(&self.tabs.tabs, path) {
                Some(index) => index,
                None => return,
            },
            None => self.tabs.active_tab,
        };
        // The buffer moved on while the formatter was running; the result is
        // stale and must not clobber newer edits.
        let Some(document) = self.tabs.tabs[index].content.as_text() else {
            return;
        };
        if document.edit_generation != generation {
            return;
        }

        match result {
            Ok(formatted) => {
                if formatted.chars().eq(document.buffer.text().chars()) {
                    self.tabs.tabs[index]
                        .content
                        .as_text_mut()
                        .unwrap()
                        .document_status = Some("Already formatted".to_string());
                    return;
                }
                let total_chars = document.buffer.text().len_chars();
                let positions = document.caret_state.caret_chars_snapshot();
                if self.tabs.tabs[index]
                    .content
                    .as_text_mut()
                    .is_some_and(|doc| doc.apply_edit(0, total_chars, &formatted))
                {
                    let Some(document) = self.tabs.tabs[index].content.as_text_mut() else {
                        return;
                    };
                    let len = document.buffer.text().len_chars();
                    let clamped = positions
                        .iter()
                        .map(|pos| (*pos).min(len))
                        .collect::<Vec<usize>>();
                    document
                        .caret_state
                        .set_all_caret_chars(&clamped, &document.buffer);
                    if from_save {
                        // The save task already wrote this text to disk, so the
                        // buffer is in the saved state, not dirty.
                        document.document_dirty = false;
                        document.saved_text = document.buffer.text().clone();
                    }
                    document.document_status = Some("Formatted".to_string());
                    self.update_window_title();
                    self.schedule_language_refresh();
                }
            }
            Err(err) => {
                self.tabs.tabs[index]
                    .content
                    .as_text_mut()
                    .unwrap()
                    .document_status = Some(format!("Format failed: {}", err));
            }
        }
    }
}
