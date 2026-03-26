mod command_panel;
mod editor;
mod events;
mod init;
mod input;
mod settings;
mod update;
mod watchers;

use crate::config::{Config, EditorSettings};
use crate::events::CustomEvent;
use crate::file_tree::FileTree;
use crate::language::HighlightingService;
use command_panel::CommandPanelState;
use editor::{CaretState, EditHistory};
use lux_core::Buffer;
use notify::RecommendedWatcher;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellView {
    Editor,
    Configuration,
}

pub struct OpenDocument {
    buffer: Buffer,
    caret_state: CaretState,
    edit_history: EditHistory,
    document_dirty: bool,
    document_status: Option<String>,
}

impl OpenDocument {
    pub fn new_empty() -> Self {
        Self {
            buffer: Buffer::new(),
            caret_state: Default::default(),
            edit_history: Default::default(),
            document_dirty: false,
            document_status: None,
        }
    }

    pub fn from_buffer(buffer: Buffer) -> Self {
        let mut doc = Self {
            buffer,
            caret_state: Default::default(),
            edit_history: Default::default(),
            document_dirty: false,
            document_status: None,
        };
        doc.caret_state.reset_to_buffer_end(&doc.buffer);
        doc
    }

    pub fn title(&self) -> String {
        let name = self
            .buffer
            .path()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(|name| name.to_string())
            .unwrap_or_else(|| "Untitled".to_string());
        if self.document_dirty {
            format!("* {}", name)
        } else {
            name
        }
    }
}

pub struct App {
    rt: tokio::runtime::Runtime,
    event_tx: Sender<CustomEvent>,
    event_rx: Receiver<CustomEvent>,
    documents: Vec<OpenDocument>,
    active_document: usize,
    workspace_path: Option<PathBuf>,
    file_tree: Option<FileTree>,
    workspace_watcher: Option<RecommendedWatcher>,
    settings_watcher: Option<RecommendedWatcher>,
    editor_config: Config,
    config_draft: EditorSettings,
    config_status: Option<String>,
    config_autosave_deadline: Option<Instant>,
    highlighting_service: HighlightingService,
    needs_style_refresh: bool,
    reveal_active_in_tree: bool,
    shell_view: ShellView,
    command_panel: CommandPanelState,
    caret_blink_anchor: Instant,
}

impl App {
    pub(in crate::app) fn active_document(&self) -> &OpenDocument {
        self.documents
            .get(self.active_document)
            .expect("active document index must be valid")
    }

    pub(in crate::app) fn active_document_mut(&mut self) -> &mut OpenDocument {
        self.documents
            .get_mut(self.active_document)
            .expect("active document index must be valid")
    }

    pub(in crate::app) fn buffer(&self) -> &Buffer {
        &self.active_document().buffer
    }

    pub(in crate::app) fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.active_document_mut().buffer
    }
}
