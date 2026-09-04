//! Workspace domain: the open folder, its file tree and the watcher that
//! refreshes them.

use super::App;
use super::document::OpenDocument;
use crate::events::{CustomEvent, WorkspaceEvent};
use crate::file_tree::FileTree;
use crate::file_watcher;
use eframe::egui;
use lux_core::Buffer;
use notify::RecommendedWatcher;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(crate) struct Workspace {
    pub(crate) path: Option<PathBuf>,
    pub(crate) file_tree: Option<FileTree>,
    pub(crate) watcher: Option<RecommendedWatcher>,
}

impl Workspace {
    pub(crate) fn start_watcher(
        workspace_path: &Path,
        event_tx: Sender<CustomEvent>,
        wake: egui::Context,
    ) -> Option<RecommendedWatcher> {
        if let Ok((watcher, rx)) = file_watcher::watch(workspace_path) {
            std::thread::spawn(move || {
                let debounce = Duration::from_millis(120);
                while let Ok(result) = rx.recv() {
                    if result.is_err() {
                        continue;
                    }
                    let batch_start = Instant::now();
                    while batch_start.elapsed() < debounce {
                        let timeout = debounce.saturating_sub(batch_start.elapsed());
                        if rx.recv_timeout(timeout).is_err() {
                            break;
                        }
                    }
                    event_tx
                        .send(CustomEvent::Workspace(WorkspaceEvent::FileChange))
                        .ok();
                    wake.request_repaint();
                }
            });
            Some(watcher)
        } else {
            None
        }
    }
}

impl App {
    pub(super) fn open_folder(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        self.workspace.path = Some(path.clone());
        self.workspace.file_tree = Some(FileTree::new(&path));
        self.settings.editor_config.add_recent(path.clone(), true);
        self.workspace.watcher = Workspace::start_watcher(
            &path,
            self.runtime.event_tx.clone(),
            self.runtime.ctx.clone(),
        );
        self.documents.reset_editor_state();
        self.open_workspace_last_file(&path);
        self.restart_settings_watcher();
        if self.settings.editor_config.reload_settings() {
            self.chrome.needs_style_refresh = true;
        }
        self.chrome
            .shell
            .sync_config_draft(&self.settings.editor_config.settings);
    }

    pub(super) fn initialize_from_path(&mut self, initial_path: Option<PathBuf>) {
        let Some(path) = initial_path else {
            return;
        };
        let path = path.canonicalize().unwrap_or(path);

        if path.is_dir() {
            self.workspace.path = Some(path.clone());
            self.workspace.file_tree = Some(FileTree::new(&path));
            self.settings.editor_config.add_recent(path.clone(), true);
            self.workspace.watcher = Workspace::start_watcher(
                &path,
                self.runtime.event_tx.clone(),
                self.runtime.ctx.clone(),
            );
            self.open_workspace_last_file(&path);
            return;
        }

        if path.is_file()
            && let Ok(buffer) = self.runtime.rt.block_on(Buffer::from_file(&path))
        {
            self.documents.tabs = vec![OpenDocument::from_buffer(buffer)];
            self.documents.active_document = 0;
            self.settings.editor_config.add_recent(path, false);
        }
    }

    pub(super) fn track_file_open(&mut self, path: &Path) {
        if let Some(workspace_path) = self
            .workspace
            .path
            .as_ref()
            .filter(|workspace_path| path.starts_with(workspace_path))
        {
            self.settings
                .editor_config
                .set_workspace_last_file(workspace_path, path);
            return;
        }
        self.settings
            .editor_config
            .add_recent(path.to_path_buf(), false);
    }

    fn open_workspace_last_file(&mut self, workspace_path: &Path) {
        let Some(path) = self
            .settings
            .editor_config
            .workspace_last_file(workspace_path)
        else {
            return;
        };
        if path.is_file()
            && let Ok(buffer) = self.runtime.rt.block_on(Buffer::from_file(&path))
        {
            self.documents.tabs = vec![OpenDocument::from_buffer(buffer)];
            self.documents.active_document = 0;
            self.refresh_language_intelligence();
        }
    }

    pub(super) fn on_file_change(&mut self) {
        if let Some(path) = &self.workspace.path {
            self.workspace.file_tree = Some(crate::file_tree::FileTree::new(path));
        }
    }
}
