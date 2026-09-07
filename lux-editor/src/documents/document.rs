//! Document model: one open file's buffer plus the editor state scoped to it
//! (carets, edit history, dirty flag, status message).

use lux_core::Buffer;
use lux_core::editor::{CaretState, EditHistory};
use std::path::PathBuf;

pub struct OpenDocument {
    pub(crate) buffer: Buffer,
    pub(crate) caret_state: CaretState,
    pub(crate) edit_history: EditHistory,
    pub(crate) document_dirty: bool,
    pub(crate) edit_generation: u64,
    pub(crate) document_status: Option<String>,
    /// The file behind `buffer.path()` is gone; the editor shows an error page.
    pub(crate) missing: bool,
}

impl OpenDocument {
    pub fn new_empty() -> Self {
        Self {
            buffer: Buffer::new(),
            caret_state: Default::default(),
            edit_history: Default::default(),
            document_dirty: false,
            edit_generation: 0,
            document_status: None,
            missing: false,
        }
    }

    /// A tab for a path that could not be read.
    pub fn missing(path: PathBuf) -> Self {
        let mut document = Self::new_empty();
        document.buffer.set_path(path);
        document.missing = true;
        document
    }

    pub fn from_buffer(buffer: Buffer) -> Self {
        let mut doc = Self {
            buffer,
            caret_state: Default::default(),
            edit_history: Default::default(),
            document_dirty: false,
            edit_generation: 0,
            document_status: None,
            missing: false,
        };
        doc.caret_state.reset_to_buffer_end(&doc.buffer);
        doc
    }

    pub fn title(&self) -> String {
        self.buffer
            .path()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    /// Re-check the file behind the tab. Returns true when the tab sits on the
    /// missing page and the file has come back, i.e. the caller must re-read it;
    /// the flag stays set until that read lands.
    /// Dirty documents are never judged, as "save as" sets a path before the
    /// bytes exist.
    pub(crate) fn observe_file_exists(&mut self, exists: bool) -> bool {
        if self.document_dirty || self.buffer.path().is_none() {
            return false;
        }
        if self.missing {
            return exists;
        }
        self.missing = !exists;
        if self.missing {
            // Whatever the status bar said last is not why the file is gone.
            self.document_status = None;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(dirty: bool, missing: bool) -> OpenDocument {
        let mut buffer = Buffer::new();
        buffer.set_path("/ws/a.rs");
        let mut document = if missing {
            OpenDocument::missing(buffer.path().unwrap().clone())
        } else {
            OpenDocument::from_buffer(buffer)
        };
        document.document_dirty = dirty;
        document
    }

    #[test]
    fn missing_flag_lands_then_asks_for_a_reload() {
        let mut opened = document(false, false);
        assert!(!opened.observe_file_exists(false));
        assert!(opened.missing);
        // The file came back: the flag holds until a re-read replaces the tab.
        assert!(opened.observe_file_exists(true));
        assert!(opened.missing);

        let mut dirty = document(true, false);
        assert!(!dirty.observe_file_exists(false));
        assert!(!dirty.missing);

        let mut untitled = OpenDocument::new_empty();
        assert!(!untitled.observe_file_exists(false));
        assert!(!untitled.missing);
    }
}
