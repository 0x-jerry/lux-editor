//! The app's event bus. UI components and background workers report their
//! effects to the reducer (`app/events.rs`) through a single `CustomEvent`
//! envelope — one variant per domain, so a new event has to pick a domain:
//!
//! - [`WorkspaceEvent`] — workspace-tree changes (create/delete/rename, refresh)
//! - [`DocumentEvent`] — document lifecycle & content pipeline (load/save/format,
//!   tabs, save & format commands)
//! - [`AppEvent`] — app-global state (config refresh), open commands, recent items
//! - [`ShellEvent`] — navigation and title-bar actions
//! - [`ConfigurationEvent`] — configuration-view autosave
//! - [`EditingEvent`] — caret/pointer interaction in the text editor

use crate::config::EditorSettings;
use std::path::PathBuf;

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

/// Document lifecycle & content pipeline: IO round-trips, tabs and the
/// save/format commands that act on the current document.
#[derive(Debug)]
pub enum DocumentEvent {
    /// An async file load finished.
    FileLoaded {
        path: PathBuf,
        buffer: Result<lux_core::Buffer, String>,
    },
    /// An async save finished.
    FileSaved {
        path: PathBuf,
        generation: u64,
        ok: bool,
    },
    /// The external formatter finished.
    FormattingFinished {
        generation: u64,
        from_save: bool,
        result: Result<String, String>,
    },
    SwitchDocument(usize),
    CloseDocument(usize),
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
    ClearRecentItems,
}

/// Shell & navigation: view switching, sidebar, command palette and title-bar menus.
#[derive(Debug)]
pub enum ShellEvent {
    SwitchToEditor,
    SwitchToConfiguration,
    ToggleSidebar,
    ToggleCommandPanel,
    TitleBarMenu(crate::app::TitleBarMenu),
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
