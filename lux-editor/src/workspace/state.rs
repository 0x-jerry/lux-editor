//! Workspace domain: the open folder, its file tree and the watcher that
//! refreshes them.

use crate::app::App;
use crate::events::{CustomEvent, DocumentEvent, ReconcileResult, WorkspaceEvent};
use crate::settings::WorkspaceSession;
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
        let tree = FileTree::new(&path);
        let root = tree.root().to_path_buf();
        self.workspace.path = Some(path.clone());
        self.workspace.file_tree = Some(tree);
        self.settings.editor_config.add_recent(path.clone(), true);
        self.workspace.watcher = Workspace::start_watcher(
            &path,
            self.runtime.event_tx.clone(),
            self.runtime.ctx.clone(),
        );
        self.documents.reset_editor_state();
        self.restore_workspace_session(&path, &root, ctx);
        self.restart_settings_watcher();
        if self.settings.editor_config.reload_settings() {
            self.chrome.needs_style_refresh = true;
        }
        self.chrome
            .shell
            .sync_config_draft(&self.settings.editor_config.settings);
    }

    /// Restore a workspace's tree expansion and tabs; one never seen before
    /// opens with just its root expanded, so the top level is already visible.
    fn restore_workspace_session(
        &mut self,
        workspace_path: &Path,
        root: &Path,
        ctx: &egui::Context,
    ) {
        let Some(session) = self
            .settings
            .editor_config
            .workspace_session(workspace_path)
            .cloned()
        else {
            self.chrome
                .shell
                .set_file_tree_expanded([root.to_path_buf()]);
            return;
        };
        self.chrome
            .shell
            .set_file_tree_expanded(session.expanded_dirs.into_iter().filter(|dir| dir.is_dir()));
        self.open_files(session.open_files, session.active_file, ctx);
    }

    /// Snapshot the live tabs and tree expansion into the workspace's session;
    /// runs every logic pass and only re-arms the debounced write on a change.
    pub(crate) fn sync_workspace_session(&mut self) {
        let Some(workspace_path) = self.workspace.path.clone() else {
            return;
        };
        // Loads in flight would otherwise blank the session between the
        // workspace opening and its restored tabs landing.
        if self.documents.pending_loads > 0 {
            return;
        }
        let inside = |path: &PathBuf| path.starts_with(&workspace_path);
        let open_files = self
            .documents
            .tabs
            .iter()
            .filter_map(|document| document.buffer.path().cloned())
            .filter(|path| inside(path))
            .collect::<Vec<_>>();
        let active_file = self
            .active_document()
            .buffer
            .path()
            .filter(|path| inside(path))
            .cloned()
            .or_else(|| {
                // A tab left over from another workspace has focus: keep the
                // remembered file while it is still open instead of erasing it.
                let remembered = self
                    .settings
                    .editor_config
                    .workspace_session(&workspace_path)?
                    .active_file
                    .clone()?;
                open_files.contains(&remembered).then_some(remembered)
            });
        let expanded_dirs = self
            .chrome
            .shell
            .file_tree_expanded()
            .iter()
            .filter(|path| inside(path))
            .cloned()
            .collect();
        self.settings
            .editor_config
            .set_workspace_session(WorkspaceSession {
                workspace_path,
                open_files,
                active_file,
                expanded_dirs,
            });
    }

    pub(crate) fn initialize_from_path(
        &mut self,
        initial_path: Option<PathBuf>,
        ctx: &egui::Context,
    ) {
        let Some(path) = initial_path else {
            return;
        };
        let path = path.canonicalize().unwrap_or(path);
        if path.is_dir() {
            self.open_folder(path, ctx);
        } else if path.is_file() {
            self.open_file(path, ctx);
        }
    }

    pub(crate) fn track_file_open(&mut self, path: &Path) {
        // In-workspace files belong to the session; recents are for the rest.
        if self
            .workspace
            .path
            .as_ref()
            .is_some_and(|workspace_path| path.starts_with(workspace_path))
        {
            return;
        }
        self.settings
            .editor_config
            .add_recent(path.to_path_buf(), false);
    }

    pub(crate) fn on_file_change(&mut self) {
        if let Some(tree) = &mut self.workspace.file_tree {
            tree.refresh();
        }
        self.sync_missing_documents();
        self.reconcile_dirty_from_disk();
    }

    /// Flag tabs whose file vanished, and re-read the ones whose file came back
    /// so the missing page cannot outlive the gap it describes. Binary tabs get
    /// the same re-read when their file is rewritten: a file that becomes valid
    /// UTF-8 again comes back as a normal tab (the loaded batch replaces the tab
    /// in place), and one that is still binary stays on the guide page.
    fn sync_missing_documents(&mut self) {
        let mut revived = Vec::new();
        for document in self.documents.tabs.iter_mut() {
            if document.binary {
                let Some(path) = document.buffer.path().cloned() else {
                    continue;
                };
                let Some(metadata) = std::fs::metadata(&path).ok() else {
                    continue;
                };
                let stat = (
                    metadata.len(),
                    metadata
                        .modified()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                );
                if document.last_disk_stat != Some(stat) {
                    revived.push(path);
                }
                continue;
            }
            let exists = document.buffer.path().is_some_and(|path| path.exists());
            if document.observe_file_exists(exists)
                && let Some(path) = document.buffer.path()
            {
                revived.push(path.clone());
            }
        }
        if !revived.is_empty() {
            self.load_files(revived, None);
        }
    }

    /// Compare every open tab's file against the buffer's bytes. The stat
    /// short-circuit runs here, on the UI thread, so a swoop of unrelated
    /// events costs only metadata calls; only files that actually moved are
    /// read, and those reads plus the byte compare run on a blocking thread.
    fn reconcile_dirty_from_disk(&mut self) {
        let mut checks = Vec::new();
        for document in self.documents.tabs.iter() {
            if document.binary || document.missing {
                continue;
            }
            let Some(path) = document.buffer.path() else {
                continue;
            };
            let Some(metadata) = std::fs::metadata(path).ok() else {
                continue;
            };
            let stat = (
                metadata.len(),
                metadata
                    .modified()
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            );
            if document.last_disk_stat == Some(stat) {
                continue;
            }
            checks.push((path.clone(), stat, document.buffer.text().to_string()));
        }
        if checks.is_empty() {
            return;
        }
        let event_tx = self.runtime.event_tx.clone();
        let wake = self.runtime.ctx.clone();
        self.runtime.rt.spawn_blocking(move || {
            let results = checks
                .into_iter()
                .map(|(path, stat, text)| {
                    let differs = std::fs::read(&path).is_ok_and(|bytes| bytes != text.as_bytes());
                    ReconcileResult {
                        path,
                        stat,
                        differs,
                    }
                })
                .collect();
            let _ = event_tx.send(CustomEvent::Document(DocumentEvent::FilesReconciled {
                results,
            }));
            wake.request_repaint();
        });
    }
}
