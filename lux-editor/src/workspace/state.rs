//! Workspace domain: the open folder, its file tree and the watcher that
//! refreshes them.

use crate::app::App;
use crate::events::{CustomEvent, WorkspaceEvent};
use crate::workspace::FileTree;
use crate::workspace::watch;
use eframe::egui;
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
        if let Ok((watcher, rx)) = watch(workspace_path) {
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
    pub(crate) fn open_folder(&mut self, path: PathBuf, ctx: &egui::Context) {
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
        self.open_workspace_last_file(&path, ctx);
        self.restart_settings_watcher();
        if self.settings.editor_config.reload_settings() {
            self.chrome.needs_style_refresh = true;
        }
        self.chrome
            .shell
            .sync_config_draft(&self.settings.editor_config.settings);
    }

    pub(crate) fn initialize_from_path(&mut self, initial_path: Option<PathBuf>, ctx: &egui::Context) {
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
            self.open_workspace_last_file(&path, ctx);
            return;
        }

        if path.is_file() {
            self.open_file(path, ctx);
        }
    }

    pub(crate) fn track_file_open(&mut self, path: &Path) {
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

    fn open_workspace_last_file(&mut self, workspace_path: &Path, ctx: &egui::Context) {
        let Some(path) = self
            .settings
            .editor_config
            .workspace_last_file(workspace_path)
        else {
            return;
        };
        if path.is_file() {
            self.open_file(path, ctx);
        }
    }

    pub(crate) fn on_file_change(&mut self) {
        if let Some(tree) = &mut self.workspace.file_tree {
            tree.refresh();
        }
    }
}
