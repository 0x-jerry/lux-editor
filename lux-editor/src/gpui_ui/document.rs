//! One open text document: a gpui-kit editor owns the text and highlighting,
//! this wrapper owns the file identity and dirty tracking around it.

use gpui_kit::component::input::{Editor, EditorState};
use gpui_kit::{Entity, Subscription};
use std::path::{Path, PathBuf};

/// Bytes scanned for a NUL when deciding whether a file is text.
const SNIFF_LEN: usize = 8192;

pub struct OpenDocument {
    pub editor: Entity<EditorState>,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    /// Keeps the change subscription alive for as long as the document is open.
    pub _changes: Subscription,
}

impl OpenDocument {
    pub fn text(&self, cx: &gpui_kit::App) -> String {
        self.editor.read(cx).value().to_string()
    }

    pub fn view(&self) -> Editor {
        Editor::new(&self.editor)
    }

    pub fn title(&self) -> String {
        self.path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_string())
    }
}

/// Whether `bytes` look binary; used to refuse opening a file as text.
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(SNIFF_LEN).any(|&byte| byte == 0)
}

/// Read a file as text, or `None` when it is binary or not valid UTF-8.
pub fn read_text(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    if is_binary(&bytes) {
        return None;
    }
    String::from_utf8(bytes).ok()
}
