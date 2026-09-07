//! Document model: one open file's buffer plus the editor state scoped to it
//! (carets, edit history, dirty flag, status message).

use lux_core::Buffer;
use lux_core::editor::{CaretState, EditHistory};
use ropey::Rope;
use std::path::PathBuf;
use std::time::SystemTime;

pub struct OpenDocument {
    pub(crate) buffer: Buffer,
    pub(crate) caret_state: CaretState,
    pub(crate) edit_history: EditHistory,
    pub(crate) document_dirty: bool,
    pub(crate) edit_generation: u64,
    pub(crate) document_status: Option<String>,
    /// The file behind `buffer.path()` is gone; the editor shows an error page.
    pub(crate) missing: bool,
    /// The file behind `buffer.path()` is binary; the editor shows a guide page
    /// and refuses to load it as text, edit it or save over it.
    pub(crate) binary: bool,
    /// Becomes true once the user clicks into this document's edit area; until
    /// then the caret stays hidden and editing is blocked. Per-document.
    pub(crate) edit_area_focused: bool,
    /// Size and mtime of the file as last loaded or saved, so the watcher can
    /// skip byte-comparing tabs whose file did not actually change.
    pub(crate) last_disk_stat: Option<(u64, SystemTime)>,
    /// The file's content as last loaded or saved: the reference the dirty flag
    /// is compared against on every edit. A rope clone, so it shares the buffer's
    /// tree instead of duplicating the text.
    pub(crate) saved_text: Rope,
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
            binary: false,
            edit_area_focused: false,
            last_disk_stat: None,
            saved_text: Rope::new(),
        }
    }

    /// A tab for a path that could not be read.
    pub fn missing(path: PathBuf) -> Self {
        let mut document = Self::new_empty();
        document.buffer.set_path(path);
        document.missing = true;
        document
    }

    /// A tab for a path whose bytes are not valid UTF-8: the editor cannot
    /// edit it, so a guide page replaces the text area instead of an empty
    /// buffer that could be saved over the real file. The stat is baselined so
    /// the watcher only re-reads the file once it is actually rewritten — that
    /// is how a file that becomes text again comes back as a normal tab.
    pub fn binary(path: PathBuf) -> Self {
        let mut document = Self::new_empty();
        document.buffer.set_path(path);
        document.binary = true;
        document.record_disk_stat();
        document
    }

    pub fn from_buffer(buffer: Buffer) -> Self {
        // Clone before moving the buffer: persistent rope, shares the tree.
        let saved_text = buffer.text().clone();
        let doc = Self {
            buffer,
            caret_state: Default::default(),
            edit_history: Default::default(),
            document_dirty: false,
            edit_generation: 0,
            document_status: None,
            missing: false,
            binary: false,
            edit_area_focused: false,
            last_disk_stat: None,
            saved_text,
        };
        // Keep the caret at the top of the file: a freshly opened document
        // shows its first lines (the editor reveals the caret on open).
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
    /// bytes exist; binary documents are never judged either, they do not turn
    /// into missing tabs no matter what happens to the file.
    pub(crate) fn observe_file_exists(&mut self, exists: bool) -> bool {
        if self.document_dirty || self.binary || self.buffer.path().is_none() {
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

    /// Record the file's current size and mtime so the watcher can tell the
    /// file changed without re-reading (and byte-comparing) it.
    pub(crate) fn record_disk_stat(&mut self) {
        self.last_disk_stat = self.buffer.path().and_then(|path| {
            std::fs::metadata(path).ok().map(|meta| {
                let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                (meta.len(), modified)
            })
        });
    }

    /// Set `document_dirty` from whether the buffer differs from the saved
    /// content, so an edit undone back to it reverts to clean. The equality
    /// check is a length gate plus a chunk scan that stops at the first
    /// differing byte; it only scans the whole file at the one moment the text
    /// genuinely matches again.
    pub(crate) fn recompute_dirty(&mut self) -> bool {
        let dirty = self.buffer.text() != &self.saved_text;
        self.document_dirty = dirty;
        dirty
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

    #[test]
    fn edit_then_revert_returns_to_clean() {
        let mut buffer = Buffer::new();
        buffer.set_path("/ws/a.rs");
        buffer.insert(0, "hello");
        let mut document = OpenDocument::from_buffer(buffer);
        assert!(!document.recompute_dirty());

        // Add a character: dirty.
        document.buffer.insert(5, "!");
        assert!(document.recompute_dirty());

        // Delete what was added: back to the saved content, clean again.
        document.buffer.remove(5..6);
        assert!(!document.recompute_dirty());

        // Same-length edits still count as different (the length gate must not
        // mask them).
        document.buffer.remove(0..1);
        document.buffer.insert(0, "H");
        assert!(document.recompute_dirty());
        assert!(document.document_dirty);
    }

    #[test]
    fn binary_document_is_never_judged_or_marked_missing() {
        let mut document = OpenDocument::binary(PathBuf::from("/ws/img.png"));
        assert!(document.binary);
        assert!(!document.missing);
        // The file exists, it is just not editable as text: the tab must not
        // flip to missing (nor ask for a reload) no matter what is on disk.
        assert!(!document.observe_file_exists(true));
        assert!(!document.missing);
        assert!(!document.observe_file_exists(false));
        assert!(!document.missing);
    }
}
