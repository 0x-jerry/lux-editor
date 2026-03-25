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
use lux_core::Buffer;
use notify::RecommendedWatcher;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

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
    highlighting_service: HighlightingService,
    needs_style_refresh: bool,
    shell_view: ShellView,
}
