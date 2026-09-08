//! Workspace domain: the open folder, its file tree and the watcher that
//! refreshes them.

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
