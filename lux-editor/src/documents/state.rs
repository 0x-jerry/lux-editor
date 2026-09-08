//! Document domain: the open tabs and the caret-blink state scoped to them.

use crate::document::DocumentBuffer;
use crate::document::OpenDocument;
use crate::events::{LoadResult, ReconcileResult};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

/// Index of the tab already showing `path`, if any.
pub(crate) fn tab_with_path(tabs: &[OpenDocument], path: &Path) -> Option<usize> {
    tabs.iter().position(|document| {
        document
            .buffer
            .path()
            .is_some_and(|existing_path| existing_path == path)
    })
}

/// Tab that holds `path`'s content. A tab on the missing-file page does not, so
/// opening that file again reads it instead of focusing the dead tab.
pub(crate) fn openable_tab(tabs: &[OpenDocument], path: &Path) -> Option<usize> {
    tab_with_path(tabs, path).filter(|&index| !tabs[index].missing)
}

pub(crate) struct Documents {
    pub(crate) tabs: Vec<OpenDocument>,
    pub(crate) active_document: usize,
    pub(crate) caret_blink_anchor: Instant,
    /// Loads handed to the runtime but not landed yet; the workspace session is
    /// frozen while this is non-zero so a restore cannot be captured as empty.
    pub(crate) pending_loads: usize,
}

impl Documents {
    const CARET_BLINK_PERIOD: std::time::Duration = std::time::Duration::from_millis(1000);

    pub(crate) fn with_empty_document() -> Self {
        Self {
            tabs: vec![OpenDocument::new_empty()],
            active_document: 0,
            caret_blink_anchor: Instant::now(),
            pending_loads: 0,
        }
    }

    pub(crate) fn active_document(&self) -> &OpenDocument {
        self.tabs
            .get(self.active_document)
            .expect("active document index must be valid")
    }

    pub(crate) fn active_document_mut(&mut self) -> &mut OpenDocument {
        self.tabs
            .get_mut(self.active_document)
            .expect("active document index must be valid")
    }

    pub(crate) fn buffer(&self) -> &DocumentBuffer {
        &self.active_document().buffer
    }

    pub(crate) fn buffer_mut(&mut self) -> &mut DocumentBuffer {
        &mut self.active_document_mut().buffer
    }

    /// The window title the active document asks for: its path with a dirty
    /// prefix, or an untitled placeholder.
    pub(crate) fn window_title(&self) -> String {
        let active_document = self.active_document();
        if let Some(path) = active_document.buffer.path() {
            let dirty_prefix = if active_document.document_dirty {
                "* "
            } else {
                ""
            };
            format!("lux - {}{}", dirty_prefix, path.display())
        } else if active_document.document_dirty {
            "lux - * Untitled".to_string()
        } else {
            "lux".to_string()
        }
    }

    pub(crate) fn reset_editor_state(&mut self) {
        let active_document = self.active_document_mut();
        active_document
            .caret_state
            .reset_to_buffer_end(&active_document.buffer);
        active_document.edit_history.clear();
        self.touch_caret_blink();
    }

    /// Land a finished load batch: an existing tab for the same path is replaced
    /// in place, new files append in read order and the first one takes over the
    /// empty untitled slot instead of adding a tab.
    pub(crate) fn apply_loaded(
        &mut self,
        entries: Vec<(PathBuf, LoadResult)>,
        activate: Option<PathBuf>,
    ) {
        for (path, result) in entries {
            let document = match result {
                LoadResult::Loaded(buffer) => {
                    let mut document = OpenDocument::from_buffer(buffer);
                    document.record_disk_stat();
                    document
                }
                LoadResult::Missing(err) => {
                    let mut document = OpenDocument::missing(path.clone());
                    document.document_status = Some(err);
                    document
                }
                LoadResult::Binary => OpenDocument::binary(path.clone()),
            };
            match tab_with_path(&self.tabs, &path) {
                Some(index) => self.tabs[index] = document,
                None if self.reuse_active_slot() => {
                    let active = self.active_document;
                    self.tabs[active] = document;
                }
                None => {
                    self.tabs.push(document);
                    self.active_document = self.tabs.len() - 1;
                }
            }
        }
        if let Some(path) = activate
            && let Some(index) = tab_with_path(&self.tabs, &path)
        {
            self.active_document = index;
        }
    }

    fn reuse_active_slot(&self) -> bool {
        if self.tabs.len() != 1 || self.active_document != 0 {
            return false;
        }
        let active_document = &self.tabs[self.active_document];
        active_document.buffer.path().is_none()
            && !active_document.document_dirty
            && active_document.buffer.text().len_chars() == 0
    }

    pub(crate) fn caret_blink_visible(&self) -> bool {
        self.caret_blink_anchor.elapsed().as_millis() % Self::CARET_BLINK_PERIOD.as_millis()
            < (Self::CARET_BLINK_PERIOD.as_millis() / 2)
    }

    /// Apply one reconcile batch (files compared against disk off the UI
    /// thread). A file whose bytes match the buffer has nothing left to save, so
    /// a stale dirty flag clears; one that differs on a clean tab is an external
    /// rewrite — the tab reloads in place instead of presenting the stale buffer
    /// as the user's unsaved work. A differing file on a tab with local edits
    /// keeps the buffer and surfaces the conflict in the status bar. Returns the
    /// paths to re-read; the loaded batch replaces those tabs in place.
    pub(crate) fn apply_reconcile_results(
        &mut self,
        results: Vec<ReconcileResult>,
    ) -> Vec<PathBuf> {
        let mut reload = Vec::new();
        for result in results {
            let Some(index) = tab_with_path(&self.tabs, &result.path) else {
                continue;
            };
            let document = &mut self.tabs[index];
            if document.binary || document.missing {
                continue;
            }
            // Baseline against the observed stat so an unchanged file is not
            // re-read on the next watcher event. Same-size rewrites inside the
            // filesystem's mtime tick stay invisible — a known ceiling, no
            // better cheap signal on offer.
            document.last_disk_stat = Some(result.stat);
            if !result.differs {
                if document.document_dirty {
                    document.document_dirty = false;
                    document.document_status = None;
                }
                continue;
            }
            if document.document_dirty {
                document.document_status = Some("File changed on disk".to_string());
            } else {
                reload.push(result.path);
            }
        }
        reload
    }

    pub(crate) fn touch_caret_blink(&mut self) {
        self.caret_blink_anchor = std::time::Instant::now();
    }

    pub(crate) fn focus_edit_area(&mut self) {
        self.active_document_mut().edit_area_focused = true;
    }

    /// Flag tabs whose file vanished, and collect the ones to re-read (the file
    /// came back, or a binary file was rewritten). Returns the paths to reload;
    /// the caller replaces those tabs in place via `load_files`.
    pub(crate) fn sync_missing_documents(&mut self) -> Vec<PathBuf> {
        let mut revived = Vec::new();
        for document in self.tabs.iter_mut() {
            if document.binary {
                let Some(path) = document.buffer.path().cloned() else {
                    continue;
                };
                let Some(metadata) = std::fs::metadata(&path).ok() else {
                    continue;
                };
                let stat = (
                    metadata.len(),
                    metadata
                        .modified()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                );
                if document.last_disk_stat != Some(stat) {
                    revived.push(path);
                }
                continue;
            }
            let exists = document.buffer.path().is_some_and(|path| path.exists());
            if document.observe_file_exists(exists)
                && let Some(path) = document.buffer.path()
            {
                revived.push(path.clone());
            }
        }
        revived
    }

    /// Plan a disk re-check of every open tab: (path, current stat, buffer
    /// text). The stat short-circuit runs here, on the UI thread, so a swoop of
    /// unrelated watcher events costs only metadata calls; only files that
    /// actually moved are returned, and the caller reads + byte-compares those
    /// on a blocking thread.
    pub(crate) fn disk_change_plan(&self) -> Vec<(PathBuf, (u64, SystemTime), String)> {
        let mut checks = Vec::new();
        for document in self.tabs.iter() {
            if document.binary || document.missing {
                continue;
            }
            let Some(path) = document.buffer.path() else {
                continue;
            };
            let Some(metadata) = std::fs::metadata(path).ok() else {
                continue;
            };
            let stat = (
                metadata.len(),
                metadata
                    .modified()
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            );
            if document.last_disk_stat == Some(stat) {
                continue;
            }
            checks.push((path.clone(), stat, document.buffer.text().to_string()));
        }
        checks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(path: &str) -> DocumentBuffer {
        let mut buffer = DocumentBuffer::new();
        buffer.set_path(path);
        buffer
    }

    #[test]
    fn loaded_batch_keeps_order_and_focuses_the_activated_tab() {
        let mut documents = Documents::with_empty_document();
        documents.apply_loaded(
            vec![
                (
                    PathBuf::from("/ws/a.rs"),
                    LoadResult::Loaded(buffer("/ws/a.rs")),
                ),
                (
                    PathBuf::from("/ws/gone.rs"),
                    LoadResult::Missing("No such file or directory".to_string()),
                ),
                (PathBuf::from("/ws/img.png"), LoadResult::Binary),
                (
                    PathBuf::from("/ws/b.rs"),
                    LoadResult::Loaded(buffer("/ws/b.rs")),
                ),
            ],
            Some(PathBuf::from("/ws/b.rs")),
        );
        assert_eq!(documents.tabs.len(), 4);
        assert_eq!(documents.active_document, 3);
        assert!(documents.tabs[1].missing);
        assert!(documents.tabs[2].binary);
        assert!(!documents.tabs[2].missing);

        assert_eq!(
            tab_with_path(&documents.tabs, Path::new("/ws/gone.rs")),
            Some(1)
        );
        assert_eq!(
            openable_tab(&documents.tabs, Path::new("/ws/gone.rs")),
            None
        );
        documents.apply_loaded(
            vec![(
                PathBuf::from("/ws/gone.rs"),
                LoadResult::Loaded(buffer("/ws/gone.rs")),
            )],
            Some(PathBuf::from("/ws/gone.rs")),
        );
        assert_eq!(documents.tabs.len(), 4);
        assert_eq!(documents.active_document, 1);
        assert!(!documents.tabs[1].missing);
        // A binary tab is openable: re-opening focuses it instead of re-reading
        // the file into a reload loop.
        assert_eq!(
            tab_with_path(&documents.tabs, Path::new("/ws/img.png")),
            Some(2)
        );
        assert_eq!(
            openable_tab(&documents.tabs, Path::new("/ws/img.png")),
            Some(2)
        );
    }

    fn stat(path: &Path) -> (u64, std::time::SystemTime) {
        let metadata = std::fs::metadata(path).unwrap();
        (
            metadata.len(),
            metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
        )
    }

    #[test]
    fn reconcile_clears_stale_dirty_and_reloads_clean_tabs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        std::fs::write(&path, "same").unwrap();

        let mut documents = Documents::with_empty_document();
        let mut buffer = DocumentBuffer::new();
        buffer.set_path(&path);
        buffer.insert(0, "same");
        documents.apply_loaded(
            vec![(path.clone(), LoadResult::Loaded(buffer))],
            Some(path.clone()),
        );
        assert!(!documents.tabs[0].document_dirty);

        // A clean tab whose file differs is an external rewrite: request a
        // reload instead of turning the stale buffer into "unsaved work".
        let reload = documents.apply_reconcile_results(vec![ReconcileResult {
            path: path.clone(),
            stat: stat(&path),
            differs: true,
        }]);
        assert_eq!(reload, vec![path.clone()]);
        assert!(!documents.tabs[0].document_dirty);

        // An edited tab whose file differs keeps the buffer and reports the
        // conflict; it is not reloaded (that would clobber the edits).
        documents.tabs[0].document_dirty = true;
        assert!(
            documents
                .apply_reconcile_results(vec![ReconcileResult {
                    path: path.clone(),
                    stat: stat(&path),
                    differs: true,
                }])
                .is_empty()
        );
        assert!(documents.tabs[0].document_dirty);
        assert_eq!(
            documents.tabs[0].document_status.as_deref(),
            Some("File changed on disk")
        );

        // The file rewritten to the buffer's bytes: nothing left to save, the
        // stale dirty flag clears.
        assert!(
            documents
                .apply_reconcile_results(vec![ReconcileResult {
                    path: path.clone(),
                    stat: stat(&path),
                    differs: false,
                }])
                .is_empty()
        );
        assert!(!documents.tabs[0].document_dirty);
    }
}
