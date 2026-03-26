use crate::app::ShellView;
use crate::config::{Config, EditorSettings};
use crate::file_tree::FileTree;
use crate::language::HighlightSnapshot;
use lux_core::Buffer;
use std::path::PathBuf;

pub struct DrawUiState<'a> {
    pub file_tree: Option<&'a FileTree>,
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a Buffer,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    pub config_draft: &'a mut EditorSettings,
    pub config_status: Option<&'a str>,
    pub shell_view: ShellView,
    pub reveal_active_in_tree: bool,
    pub caret_line: usize,
    pub caret_column: usize,
    pub selection_len: usize,
    pub caret_visible: bool,
}
