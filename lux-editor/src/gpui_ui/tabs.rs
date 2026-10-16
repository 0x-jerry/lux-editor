//! The open-tab list: stable ids, the active tab, and content lookup. A tab
//! holds a text document, a read-only file view or the settings page.

use super::document::OpenDocument;
use super::views::Notice;
use std::path::{Path, PathBuf};

/// What a tab shows.
pub enum TabContent {
    /// An editable text document.
    Text(OpenDocument),
    /// An image file, drawn by the viewer.
    Image(PathBuf),
    /// A file the viewer cannot render, with the reason.
    Notice { path: PathBuf, kind: Notice },
    /// The application settings page; the view itself lives in the root.
    Settings,
}

pub struct Tab {
    pub id: u64,
    pub content: TabContent,
}

impl Tab {
    /// The text document in this tab, if it holds one.
    pub fn text(&self) -> Option<&OpenDocument> {
        match &self.content {
            TabContent::Text(doc) => Some(doc),
            _ => None,
        }
    }

    pub fn text_mut(&mut self) -> Option<&mut OpenDocument> {
        match &mut self.content {
            TabContent::Text(doc) => Some(doc),
            _ => None,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.content {
            TabContent::Text(doc) => doc.path.as_deref(),
            TabContent::Image(path) | TabContent::Notice { path, .. } => Some(path),
            TabContent::Settings => None,
        }
    }

    pub fn title(&self) -> String {
        match &self.content {
            TabContent::Text(doc) => doc.title(),
            TabContent::Image(path) | TabContent::Notice { path, .. } => file_name(path),
            TabContent::Settings => "Settings".to_string(),
        }
    }

    pub fn dirty(&self) -> bool {
        matches!(&self.content, TabContent::Text(doc) if doc.dirty)
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
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

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// The index of the tab showing `path`, whatever kind of content it holds.
    pub fn index_of(&self, path: &Path) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.path() == Some(path))
    }

    pub fn settings_index(&self) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| matches!(tab.content, TabContent::Settings))
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

    pub fn tab(&self, index: usize) -> Option<&Tab> {
        self.tabs.get(index)
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    pub fn active_doc(&self) -> Option<&OpenDocument> {
        self.tabs.get(self.active).and_then(Tab::text)
    }

    pub fn active_doc_mut(&mut self) -> Option<&mut OpenDocument> {
        self.tabs.get_mut(self.active).and_then(Tab::text_mut)
    }

    pub fn active_index(&self) -> Option<usize> {
        (!self.tabs.is_empty()).then_some(self.active)
    }

    pub fn get(&self, index: usize) -> Option<&OpenDocument> {
        self.tabs.get(index).and_then(Tab::text)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut OpenDocument> {
        self.tabs.get_mut(index).and_then(Tab::text_mut)
    }

    pub fn id_at(&self, index: usize) -> Option<u64> {
        self.tabs.get(index).map(|tab| tab.id)
    }

    pub fn is_dirty(&self, index: usize) -> bool {
        self.tabs.get(index).is_some_and(Tab::dirty)
    }

    pub fn mark_dirty(&mut self, id: u64) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) {
            if let Some(doc) = tab.text_mut() {
                doc.dirty = true;
            }
        }
    }

    pub fn paths(&self) -> Vec<PathBuf> {
        self.tabs
            .iter()
            .filter_map(|tab| tab.path().map(Path::to_path_buf))
            .collect()
    }
}
