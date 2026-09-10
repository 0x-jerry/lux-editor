//! The app's event bus. UI components and background workers report their
//! effects to the app's actions (`app/actions`, `impl Ctx` methods) through a
//! single `CustomEvent` envelope — one variant per domain, so a new event has
//! to pick a domain:
//!
//! - [`WorkspaceEvent`] — workspace-tree changes (create/delete/rename, refresh)
//! - [`DocumentEvent`] — document lifecycle & content pipeline (load/save/format,
//!   tab switching/closing, save & format commands)
//! - [`AppEvent`] — app-global state (config refresh), open commands, recent items
//! - [`ShellEvent`] — navigation and title-bar actions
//! - [`ConfigurationEvent`] — configuration-view autosave
//! - [`EditingEvent`] — caret/pointer interaction in the text editor

use crate::document::DocumentBuffer;
use crate::settings::EditorSettings;
use std::path::PathBuf;
use std::time::SystemTime;

/// Workspace: changes to the open workspace tree — raw mutations from the
/// file-tree actions plus the refresh triggered by the file watcher.
#[derive(Debug)]
pub enum WorkspaceEvent {
    /// A watched file changed; the workspace tree needs rebuilding.
    FileChange,
    Delete(PathBuf),
    Rename(PathBuf, PathBuf),
    NewFile(PathBuf),
    NewFolder(PathBuf),
}

/// Outcome of reading one file into a tab: a loaded text buffer, a missing
/// file, or a binary file the editor refuses to load as text.
#[derive(Debug)]
pub enum LoadResult {
    Loaded(DocumentBuffer),
    Missing(String),
    Binary,
}

/// One tab compared against its file's bytes after a watcher event.
/// `stat` is the size+mtime the comparison observed, so the tab can baseline
/// against it and skip a re-read until the file moves again.
#[derive(Debug)]
pub struct ReconcileResult {
    pub path: PathBuf,
    pub stat: (u64, SystemTime),
    /// The file's bytes differ from the buffer's text.
    pub differs: bool,
}

/// Document lifecycle & content pipeline: IO round-trips, tabs and the
/// save/format commands that act on the current document.
#[derive(Debug)]
pub enum DocumentEvent {
    /// Loads finished, in the order they were requested.
    FilesLoaded {
        entries: Vec<(PathBuf, LoadResult)>,
        /// Tab to focus once the batch lands.
        activate: Option<PathBuf>,
        /// Workspace the batch was requested for; a stale batch is dropped.
        workspace: Option<PathBuf>,
    },
    /// An async save finished.
    FileSaved {
        path: PathBuf,
        generation: u64,
        ok: bool,
    },
    /// The external formatter finished.
    FormattingFinished {
        /// The file the formatter ran on; `None` for an untitled buffer, which
        /// the handler resolves to the active tab. Guards against applying a
        /// result to whatever tab happens to be active now.
        path: Option<PathBuf>,
        generation: u64,
        from_save: bool,
        result: Result<String, String>,
    },
    /// Open files compared against their bytes on disk after a watcher event;
    /// the tabs action reacts per tab (clear stale dirty / flag or reload).
    FilesReconciled {
        results: Vec<ReconcileResult>,
    },
    /// Focus the tab with this id (any content type).
    SwitchTab(u64),
    /// Close the tab with this id. A dirty tab opens the save/discard prompt
    /// instead of closing outright.
    CloseTab(u64),
    /// The close prompt chose Save: save the tab, then close it once the write
    /// lands clean.
    SaveAndCloseTab(u64),
    /// The close prompt chose Discard: close the tab without saving.
    DiscardTab(u64),
    SaveFile,
    FormatFile,
}

/// App-level state & navigation: config refresh from disk, the open commands
/// and the whole-app recent-items list.
#[derive(Debug)]
pub enum AppEvent {
    /// The user's settings file changed on disk.
    ConfigChange,
    OpenFile(PathBuf),
    OpenFolder(PathBuf),
    /// Open (or focus) the configuration tab.
    OpenConfiguration,
    ClearRecentItems,
}

/// Shell & navigation: sidebar, command palette and title-bar menus. View
/// switching lives in the tab domain (`DocumentEvent`) now.
#[derive(Debug)]
// Every shell action is a toggle; the shared prefix is the convention.
#[allow(clippy::enum_variant_names)]
pub enum ShellEvent {
    ToggleSidebar,
    ToggleCommandPanel,
    ToggleMarkdownPreview,
}

/// Configuration: the configuration view autosaving its draft.
#[derive(Debug)]
pub enum ConfigurationEvent {
    ConfigurationSaved(EditorSettings),
}

/// Text editing & caret: pointer interaction in the text editor.
#[derive(Debug)]
pub enum EditingEvent {
    SetCaretFromPointer {
        line_index: usize,
        column: usize,
        selecting: bool,
        add_cursor: bool,
    },
    SelectWordFromPointer {
        line_index: usize,
        column: usize,
    },
}

/// The event-bus envelope: one variant per domain. Components whose effect
/// surface spans several domains (`Shell`, `AppView`, `CommandPanel`,
/// `FileTreePanel`, `EditorView`) use this as their `Component::Message`;
/// single-domain components use their domain enum directly.
#[derive(Debug)]
pub enum CustomEvent {
    Workspace(WorkspaceEvent),
    Document(DocumentEvent),
    App(AppEvent),
    Shell(ShellEvent),
    Configuration(ConfigurationEvent),
    Editing(EditingEvent),
}
