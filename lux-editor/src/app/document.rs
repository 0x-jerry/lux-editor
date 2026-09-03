//! Document model: one open file's buffer plus the editor state scoped to it
//! (carets, edit history, dirty flag, status message).

use lux_core::Buffer;
use lux_core::editor::{CaretState, EditHistory};

pub struct OpenDocument {
    pub(crate) buffer: Buffer,
    pub(crate) caret_state: CaretState,
    pub(crate) edit_history: EditHistory,
    pub(crate) document_dirty: bool,
    pub(crate) edit_generation: u64,
    pub(crate) document_status: Option<String>,
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
        }
    }

    pub fn from_buffer(buffer: Buffer) -> Self {
        let mut doc = Self {
            buffer,
            caret_state: Default::default(),
            edit_history: Default::default(),
            document_dirty: false,
            edit_generation: 0,
            document_status: None,
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
}
