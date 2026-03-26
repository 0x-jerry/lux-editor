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

pub struct App {
    rt: tokio::runtime::Runtime,
    event_tx: Sender<CustomEvent>,
    event_rx: Receiver<CustomEvent>,
    buffer: Buffer,
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
    caret_state: CaretState,
    edit_history: EditHistory,
    caret_blink_anchor: Instant,
}
