//! Tab domain: the generic tab host. Tabs hold content of any kind (text
//! documents today, the configuration form next) and the strip renders them
//! without knowing what they contain. Text-specific machinery lives in
//! [`crate::document`]; this module only manages the list, stable ids, the
//! active tab and the caret-blink clock the text editor borrows.

use crate::document::OpenDocument;
use crate::events::{LoadResult, ReconcileResult};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// One open tab: a stable id plus its content.
pub struct Tab {
    pub(crate) id: u64,
    pub(crate) content: TabContent,
}

impl Tab {
    fn text(id: u64, document: OpenDocument) -> Self {
        Self {
            id,
            content: TabContent::Text(document),
        }
    }

    /// Content-agnostic metadata the shared tab strip renders.
    pub(crate) fn meta(&self) -> TabMeta {
        TabMeta {
            id: self.id,
            title: self.content.title(),
            dirty: self.content.is_dirty(),
            closable: true,
            state: self.content.tab_state(),
        }
    }
}

/// What a tab holds. Sealed; extend with a variant for each new content type.
#[allow(clippy::large_enum_variant)] // Text is the common container; boxing it would hurt every access
pub enum TabContent {
    /// A text file: buffer, carets, history, dirty state.
    Text(OpenDocument),
    /// The settings form. Its session state (draft, autosave) lives on the
    /// shell; the tab is only the marker that it is open.
    Configuration,
}

impl TabContent {
    pub(crate) fn as_text(&self) -> Option<&OpenDocument> {
        match self {
            TabContent::Text(document) => Some(document),
            TabContent::Configuration => None,
        }
    }

    pub(crate) fn as_text_mut(&mut self) -> Option<&mut OpenDocument> {
        match self {
            TabContent::Text(document) => Some(document),
            TabContent::Configuration => None,
        }
    }

    fn title(&self) -> String {
        match self {
            TabContent::Text(document) => document.title(),
            TabContent::Configuration => "Configuration".to_string(),
        }
    }

    fn is_dirty(&self) -> bool {
        match self {
            TabContent::Text(document) => document.document_dirty,
            TabContent::Configuration => false,
        }
    }

    fn tab_state(&self) -> TabState {
        match self {
            TabContent::Text(document) if document.missing => TabState::Warning,
            _ => TabState::Normal,
        }
    }
}

/// Content-agnostic tab metadata for the strip.
pub struct TabMeta {
    pub id: u64,
    pub title: String,
    pub dirty: bool,
    pub closable: bool,
    pub state: TabState,
}

/// Neutral rendering tone; each content maps its own conditions onto it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TabState {
    Normal,
    /// The backing resource is gone/unavailable (e.g. a deleted file).
    Warning,
}

/// Index of the tab already showing `path`, if any.
pub(crate) fn tab_with_path(tabs: &[Tab], path: &Path) -> Option<usize> {
    tabs.iter().position(|tab| {
        tab.content
            .as_text()
            .and_then(|document| document.buffer.path())
            .is_some_and(|existing_path| existing_path == path)
    })
}

/// Tab that holds `path`'s content. A tab on the missing-file page does not, so
/// opening that file again reads it instead of focusing the dead tab.
pub(crate) fn openable_tab(tabs: &[Tab], path: &Path) -> Option<usize> {
    tab_with_path(tabs, path)
        .filter(|&index| !tabs[index].content.as_text().is_some_and(|d| d.missing))
}

pub(crate) struct TabManager {
    pub(crate) tabs: Vec<Tab>,
    pub(crate) active_tab: usize,
    next_id: u64,
    /// The text tab focused before the configuration tab was opened, so
    /// returning to the editor restores it.
    previous_text_tab: Option<u64>,
    /// Loads handed to the runtime but not landed yet; the workspace session is
    /// frozen while this is non-zero so a restore cannot be captured as empty.
    pub(crate) pending_loads: usize,
}

impl TabManager {
    pub(crate) fn with_empty_document() -> Self {
        Self {
            tabs: vec![Tab::text(0, OpenDocument::new_empty())],
            active_tab: 0,
            next_id: 1,
            previous_text_tab: None,
            pending_loads: 0,
        }
    }

    pub(crate) fn active_is_configuration(&self) -> bool {
        matches!(
            self.tabs.get(self.active_tab).map(|tab| &tab.content),
            Some(TabContent::Configuration)
        )
    }

    pub(crate) fn has_configuration_tab(&self) -> bool {
        self.tabs
            .iter()
            .any(|tab| matches!(tab.content, TabContent::Configuration))
    }

    /// Create the configuration tab on demand, or focus the existing one. The
    /// tab is a singleton: there is never more than one.
    pub(crate) fn open_configuration(&mut self) {
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| matches!(tab.content, TabContent::Configuration))
        {
            self.active_tab = index;
            return;
        }
        if self.tabs[self.active_tab].content.as_text().is_some() {
            self.previous_text_tab = Some(self.tabs[self.active_tab].id);
        }
        self.tabs.push(Tab {
            id: self.next_id,
            content: TabContent::Configuration,
        });
        self.next_id += 1;
        self.active_tab = self.tabs.len() - 1;
    }

    /// Focus the tab with `id`; a focused text tab is remembered so closing the
    /// configuration tab can return to it.
    pub(crate) fn focus_tab(&mut self, id: u64) -> bool {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return false;
        };
        if self.tabs[index].content.as_text().is_some() {
            self.previous_text_tab = Some(id);
        }
        self.active_tab = index;
        true
    }

    /// Leave the configuration tab back to the last focused text tab (or the
    /// first one if it is gone).
    pub(crate) fn focus_previous_text(&mut self) {
        let target = self
            .previous_text_tab
            .and_then(|id| {
                self.tabs.iter().position(|tab| {
                    tab.id == id && tab.content.as_text().is_some()
                })
            })
            .or_else(|| {
                self.tabs
                    .iter()
                    .position(|tab| tab.content.as_text().is_some())
            });
        if let Some(index) = target {
            self.active_tab = index;
        }
    }

    /// Remove the tab at `index`. Last tab resets the strip to a fresh
    /// scratch text tab; closing the configuration tab returns to the last
    /// focused text tab.
    pub(crate) fn remove_tab(&mut self, index: usize) {
        let removed_is_configuration = matches!(self.tabs[index].content, TabContent::Configuration);
        let removed_id = self.tabs[index].id;
        if self.tabs.len() == 1 {
            self.tabs.clear();
            self.tabs
                .push(Tab::text(self.next_id, OpenDocument::new_empty()));
            self.next_id += 1;
            self.active_tab = 0;
            self.previous_text_tab = None;
            return;
        }
        self.tabs.remove(index);
        if removed_is_configuration {
            self.focus_previous_text();
            self.previous_text_tab = None;
            return;
        }
        if !self.tabs.iter().any(|tab| tab.content.as_text().is_some()) {
            // Only the configuration tab would be left; the strip always keeps
            // one text tab, so drop it and reset to a fresh scratch.
            self.tabs.clear();
            self.tabs
                .push(Tab::text(self.next_id, OpenDocument::new_empty()));
            self.next_id += 1;
            self.active_tab = 0;
            self.previous_text_tab = None;
            return;
        }
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        } else if index < self.active_tab {
            self.active_tab -= 1;
        }
        if self.previous_text_tab == Some(removed_id) {
            self.previous_text_tab = None;
        }
    }

    /// The first text tab in the strip; there is always one (the scratch
    /// tab), so the configuration page can name a buffer even while active.
    pub(crate) fn first_text(&self) -> Option<&OpenDocument> {
        self.tabs.iter().find_map(|tab| tab.content.as_text())
    }

    pub(crate) fn active_text(&self) -> Option<&OpenDocument> {
        self.tabs
            .get(self.active_tab)
            .and_then(|tab| tab.content.as_text())
    }

    pub(crate) fn active_text_mut(&mut self) -> Option<&mut OpenDocument> {
        self.tabs
            .get_mut(self.active_tab)
            .and_then(|tab| tab.content.as_text_mut())
    }

    pub(crate) fn active_document(&self) -> &OpenDocument {
        self.active_text().expect("active tab must be a text tab")
    }

    /// The window title the active tab asks for: its path with a dirty prefix,
    /// an untitled placeholder, or the configuration page.
    pub(crate) fn window_title(&self) -> String {
        match &self.tabs[self.active_tab].content {
            TabContent::Text(active_document) => {
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
            TabContent::Configuration => "lux - Configuration".to_string(),
        }
    }

    pub(crate) fn reset_editor_state(&mut self) {
        let Some(active_document) = self.active_text_mut() else {
            return;
        };
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
                Some(index) => self.tabs[index].content = TabContent::Text(document),
                None if self.reuse_active_slot() => {
                    let active = self.active_tab;
                    self.tabs[active].content = TabContent::Text(document);
                }
                None => {
                    self.tabs.push(Tab::text(self.next_id, document));
                    self.next_id += 1;
                    self.active_tab = self.tabs.len() - 1;
                }
            }
        }
        if let Some(path) = activate
            && let Some(index) = tab_with_path(&self.tabs, &path)
        {
            self.active_tab = index;
        }
    }

    fn reuse_active_slot(&self) -> bool {
        if self.tabs.len() != 1 || self.active_tab != 0 {
            return false;
        }
        let active_document = self.active_document();
        active_document.buffer.path().is_none()
            && !active_document.document_dirty
            && active_document.buffer.text().len_chars() == 0
    }

    pub(crate) fn caret_blink_visible(&self) -> bool {
        self.active_text().is_none_or(|document| document.caret_blink_visible())
    }

    pub(crate) fn touch_caret_blink(&mut self) {
        if let Some(document) = self.active_text_mut() {
            document.touch_caret_blink();
        }
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
            let Some(document) = self.tabs[index].content.as_text_mut() else {
                continue;
            };
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

    /// Flag tabs whose file vanished, and collect the ones to re-read (the file
    /// came back, or a binary file was rewritten). Returns the paths to reload;
    /// the caller replaces those tabs in place via `load_files`.
    pub(crate) fn sync_missing_documents(&mut self) -> Vec<PathBuf> {
        let mut revived = Vec::new();
        for tab in self.tabs.iter_mut() {
            let Some(document) = tab.content.as_text_mut() else {
                continue;
            };
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
        for tab in self.tabs.iter() {
            let Some(document) = tab.content.as_text() else {
                continue;
            };
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
    use crate::document::DocumentBuffer;

    fn buffer(path: &str) -> DocumentBuffer {
        let mut buffer = DocumentBuffer::new();
        buffer.set_path(path);
        buffer
    }

    #[test]
    fn loaded_batch_keeps_order_and_focuses_the_activated_tab() {
        let mut manager = TabManager::with_empty_document();
        manager.apply_loaded(
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
        assert_eq!(manager.tabs.len(), 4);
        assert_eq!(manager.active_tab, 3);
        assert!(manager.tabs[1].content.as_text().unwrap().missing);
        assert!(manager.tabs[2].content.as_text().unwrap().binary);
        assert!(!manager.tabs[2].content.as_text().unwrap().missing);

        assert_eq!(
            tab_with_path(&manager.tabs, Path::new("/ws/gone.rs")),
            Some(1)
        );
        assert_eq!(
            openable_tab(&manager.tabs, Path::new("/ws/gone.rs")),
            None
        );
        manager.apply_loaded(
            vec![(
                PathBuf::from("/ws/gone.rs"),
                LoadResult::Loaded(buffer("/ws/gone.rs")),
            )],
            Some(PathBuf::from("/ws/gone.rs")),
        );
        assert_eq!(manager.tabs.len(), 4);
        assert_eq!(manager.active_tab, 1);
        assert!(!manager.tabs[1].content.as_text().unwrap().missing);
        // A binary tab is openable: re-opening focuses it instead of re-reading
        // the file into a reload loop.
        assert_eq!(
            tab_with_path(&manager.tabs, Path::new("/ws/img.png")),
            Some(2)
        );
        assert_eq!(
            openable_tab(&manager.tabs, Path::new("/ws/img.png")),
            Some(2)
        );
    }

    #[test]
    fn configuration_tab_is_created_on_demand_and_destroyed_on_close() {
        let mut manager = TabManager::with_empty_document();
        assert!(!manager.has_configuration_tab());
        manager.open_configuration();
        assert!(manager.has_configuration_tab());
        assert!(manager.active_is_configuration());
        assert_eq!(manager.tabs.len(), 2);
        // Singleton: opening again focuses the existing tab, no duplicate.
        manager.open_configuration();
        assert_eq!(manager.tabs.len(), 2);

        // Returning to the editor restores the text tab.
        manager.focus_previous_text();
        assert!(!manager.active_is_configuration());

        // Opening then closing the configuration tab destroys it.
        manager.open_configuration();
        let config_index = manager
            .tabs
            .iter()
            .position(|tab| matches!(tab.content, TabContent::Configuration))
            .unwrap();
        manager.remove_tab(config_index);
        assert!(!manager.has_configuration_tab());
        assert_eq!(manager.tabs.len(), 1);
        assert!(!manager.active_is_configuration());
    }

    #[test]
    fn closing_the_last_text_tab_while_configuration_open_resets_to_scratch() {
        let mut manager = TabManager::with_empty_document();
        manager.open_configuration();
        assert_eq!(manager.tabs.len(), 2);

        // Closing the only text tab must never leave a configuration-only
        // strip: fall back to a fresh scratch text tab.
        manager.remove_tab(0);
        assert_eq!(manager.tabs.len(), 1);
        assert!(manager.tabs[0].content.as_text().is_some());
        assert!(!manager.has_configuration_tab());
        assert_eq!(manager.active_tab, 0);
    }

    #[test]
    fn tab_ids_are_unique_and_survive_in_place_replacement() {
        let mut manager = TabManager::with_empty_document();
        manager.apply_loaded(
            vec![(
                PathBuf::from("/ws/a.rs"),
                LoadResult::Loaded(buffer("/ws/a.rs")),
            )],
            None,
        );
        manager.open_configuration();
        let sketch_tab = manager.tabs[0].id;
        let mut ids = manager.tabs.iter().map(|tab| tab.id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), manager.tabs.len());

        // Replacing the tab's content in place keeps its id.
        manager.apply_loaded(
            vec![(
                PathBuf::from("/ws/a.rs"),
                LoadResult::Loaded(buffer("/ws/a.rs")),
            )],
            None,
        );
        assert_eq!(manager.tabs[0].id, sketch_tab);
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

        let mut manager = TabManager::with_empty_document();
        let mut buffer = DocumentBuffer::new();
        buffer.set_path(&path);
        buffer.insert(0, "same");
        manager.apply_loaded(
            vec![(path.clone(), LoadResult::Loaded(buffer))],
            Some(path.clone()),
        );
        assert!(!manager.tabs[0].content.as_text().unwrap().document_dirty);

        // A clean tab whose file differs is an external rewrite: request a
        // reload instead of turning the stale buffer into "unsaved work".
        let reload = manager.apply_reconcile_results(vec![ReconcileResult {
            path: path.clone(),
            stat: stat(&path),
            differs: true,
        }]);
        assert_eq!(reload, vec![path.clone()]);
        assert!(!manager.tabs[0].content.as_text().unwrap().document_dirty);

        // An edited tab whose file differs keeps the buffer and reports the
        // conflict; it is not reloaded (that would clobber the edits).
        manager.tabs[0].content.as_text_mut().unwrap().document_dirty = true;
        assert!(
            manager
                .apply_reconcile_results(vec![ReconcileResult {
                    path: path.clone(),
                    stat: stat(&path),
                    differs: true,
                }])
                .is_empty()
        );
        assert!(manager.tabs[0].content.as_text().unwrap().document_dirty);
        assert_eq!(
            manager.tabs[0]
                .content
                .as_text()
                .unwrap()
                .document_status
                .as_deref(),
            Some("File changed on disk")
        );

        // The file rewritten to the buffer's bytes: nothing left to save, the
        // stale dirty flag clears.
        assert!(
            manager
                .apply_reconcile_results(vec![ReconcileResult {
                    path: path.clone(),
                    stat: stat(&path),
                    differs: false,
                }])
                .is_empty()
        );
        assert!(!manager.tabs[0].content.as_text().unwrap().document_dirty);
    }
}