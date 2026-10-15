//! The open-tab list: stable ids, the active tab, and content lookup. Text
//! documents today; the configuration tab joins later.

use super::document::OpenDocument;
use std::path::{Path, PathBuf};

pub struct Tab {
    pub id: u64,
    pub doc: OpenDocument,
}

#[derive(Default)]
pub struct Tabs {
    pub tabs: Vec<Tab>,
    pub active: usize,
    next_id: u64,
}

impl Tabs {
    pub fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn index_of(&self, path: &Path) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.doc.path.as_deref() == Some(path))
    }

    pub fn push(&mut self, tab: Tab) {
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
    }

    pub fn close(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.active >= self.tabs.len() && !self.tabs.is_empty() {
            self.active = self.tabs.len() - 1;
        }
    }

    pub fn active_doc(&self) -> Option<&OpenDocument> {
        self.tabs.get(self.active).map(|tab| &tab.doc)
    }

    pub fn active_doc_mut(&mut self) -> Option<&mut OpenDocument> {
        self.tabs.get_mut(self.active).map(|tab| &mut tab.doc)
    }

    pub fn mark_dirty(&mut self, id: u64) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) {
            tab.doc.dirty = true;
        }
    }

    pub fn paths(&self) -> Vec<PathBuf> {
        self.tabs
            .iter()
            .filter_map(|tab| tab.doc.path.clone())
            .collect()
    }
}
