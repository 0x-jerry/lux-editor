//! Per-frame input snapshots grouped by domain. Shared by the shell and the
//! editor view so every layer speaks the same domain vocabulary.

use crate::document::DocumentBuffer;
use crate::highlighting::HighlightSnapshot;
use crate::tabs::TabMeta;
use crate::workspace::FileTree;
use std::ops::Range;
use std::path::PathBuf;

/// Sidebar domain: the workspace file tree, `None` without a workspace.
pub struct SidebarInput<'a> {
    pub file_tree: Option<&'a mut FileTree>,
}

/// Workspace domain: the open folder and session-restore progress.
pub struct WorkspaceInput<'a> {
    pub path: Option<&'a PathBuf>,
    /// Still reading the files a restored workspace remembered; the welcome page waits.
    pub restoring_session: bool,
}

/// Tab domain: the tab strip and what the active tab is.
pub struct TabsInput<'a> {
    pub tabs: &'a [TabMeta],
    pub active_id: u64,
    pub active_is_configuration: bool,
    /// The active tab is a markdown document (and not the configuration tab).
    pub active_is_markdown: bool,
}

/// Document domain: the active buffer and its on-disk state.
pub struct DocumentInput<'a> {
    pub buffer: &'a DocumentBuffer,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub status: Option<&'a str>,
    pub dirty: bool,
    pub missing: bool,
    pub binary: bool,
    /// Size of the active tab's file on disk as last loaded or saved.
    pub file_size: Option<u64>,
}

/// Caret domain: cursor positions and selections.
pub struct CaretInput<'a> {
    /// All cursor positions as 1-based (line, column).
    pub carets: &'a [(usize, usize)],
    pub selection_ranges: &'a [Range<usize>],
    pub active_index: usize,
    pub visible: bool,
}
