use crate::app::ShellView;
use crate::config::{Config, EditorSettings};
use crate::file_tree::FileTree;
use crate::language::HighlightSnapshot;
use lux_core::Buffer;
use std::ops::Range;
use std::path::PathBuf;

pub struct DocumentTab {
    pub title: String,
    pub dirty: bool,
}

pub struct DrawUiState<'a> {
    pub file_tree: Option<&'a FileTree>,
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a Buffer,
    pub document_tabs: &'a [DocumentTab],
    pub active_document_index: usize,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    pub config_draft: &'a mut EditorSettings,
    pub config_status: Option<&'a str>,
    pub document_status: Option<&'a str>,
    pub shell_view: ShellView,
    pub sidebar_visible: bool,
    pub reveal_active_in_tree: bool,
    /// All cursor positions as 1-based (line, column).
    pub carets: Vec<(usize, usize)>,
    pub selection_ranges: Vec<Range<usize>>,
    pub active_caret_index: usize,
    pub caret_visible: bool,
    pub document_dirty: bool,
}
