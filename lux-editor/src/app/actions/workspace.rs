//! Workspace actions: opening a folder, restoring and persisting its session,
//! and reacting to file-system changes (watcher events, tree mutations). The
//! pure per-tab checks live on `TabManager`; these `impl Ctx` methods add the
//! cross-domain effects (session store, chrome shell expansion, reloads).

use crate::app::Ctx;
use crate::events::{CustomEvent, DocumentEvent, ReconcileResult, WorkspaceEvent};
use crate::settings::WorkspaceSession;
use crate::workspace::FileTree;
use crate::workspace::Workspace;
use std::path::{Path, PathBuf};

impl Ctx<'_> {
    /// File system & workspace: raw mutations of the workspace tree.
    pub(crate) fn handle_workspace_event(&mut self, event: WorkspaceEvent) {
        match event {
            WorkspaceEvent::FileChange => self.on_file_change(),
            WorkspaceEvent::Delete(path) => {
                if path.is_dir() {
                    std::fs::remove_dir_all(path).ok();
                } else {
                    std::fs::remove_file(path).ok();
                }
                self.on_file_change();
            }
            WorkspaceEvent::Rename(old, new) => {
                if std::fs::rename(&old, &new).is_ok() {
                    self.on_path_renamed(&old, &new);
                }
                self.on_file_change();
            }
            WorkspaceEvent::NewFile(parent) => {
                std::fs::File::create(parent.join("new_file.txt")).ok();
                self.on_file_change();
            }
            WorkspaceEvent::NewFolder(parent) => {
                std::fs::create_dir(parent.join("new_folder")).ok();
                self.on_file_change();
            }
        }
    }

    pub(crate) fn open_folder(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        let exclude = self.settings.editor_config.settings.explorer.exclude.clone();
        let tree = FileTree::new(&path, &exclude);
        let root = tree.root().to_path_buf();
        self.workspace.path = Some(path.clone());
        self.workspace.file_tree = Some(tree);
        self.settings.editor_config.add_recent(path.clone(), true);
        self.workspace.watcher = Workspace::start_watcher(
            &path,
            &exclude,
            self.runtime.event_tx.clone(),
            self.egui_ctx().clone(),
        );
        self.tabs.reset_editor_state();
        self.restore_workspace_session(&path, &root);
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
    fn restore_workspace_session(&mut self, workspace_path: &Path, root: &Path) {
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
        self.open_files(session.open_files, session.active_file);
        // The configuration tab was focused when the session was saved. Restore
        // it after the remembered file gets focus (they are loaded async), so
        // "editor" from there finds that file again.
        if session.configuration_open {
            if self.tabs.pending_loads > 0 {
                self.frame.pending_configuration_restore = Some(workspace_path.to_path_buf());
            } else {
                self.open_configuration_tab();
            }
        }
    }

    /// Push the configured exclude patterns into the open tree. `set_excluded`
    /// is a no-op while they are unchanged, so this can run every logic pass
    /// and still catch an autosave, an external config edit and a restore.
    /// A real change rebuilds the watcher too: it filters events with the
    /// patterns it captured when the folder opened, so dropping one would
    /// otherwise leave the newly visible paths stale.
    pub(crate) fn sync_tree_exclude(&mut self) {
        let exclude = &self.settings.editor_config.settings.explorer.exclude;
        let Some(tree) = &mut self.workspace.file_tree else {
            return;
        };
        if !tree.set_excluded(exclude) {
            return;
        }
        let Some(root) = self.workspace.path.clone() else {
            return;
        };
        self.workspace.watcher = Workspace::start_watcher(
            &root,
            exclude,
            self.runtime.event_tx.clone(),
            self.egui_ctx().clone(),
        );
    }

    /// Snapshot the live tabs and tree expansion into the workspace's session;
    /// runs every logic pass and only re-arms the debounced write on a change.
    pub(crate) fn sync_workspace_session(&mut self) {
        let Some(workspace_path) = self.workspace.path.clone() else {
            return;
        };
        // Loads in flight would otherwise blank the session between the
        // workspace opening and its restored tabs landing.
        if self.tabs.pending_loads > 0 {
            return;
        }
        let inside = |path: &PathBuf| path.starts_with(&workspace_path);
        let open_files = self
            .tabs
            .tabs
            .iter()
            .filter_map(|tab| tab.content.as_text())
            .filter_map(|document| document.buffer.path().cloned())
            .filter(|path| inside(path))
            .collect::<Vec<_>>();
        let active_file = self
            .tabs
            .active_text()
            .and_then(|document| document.buffer.path())
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
                configuration_open: self.tabs.active_is_configuration(),
                expanded_dirs,
            });
    }

    pub(crate) fn initialize_from_path(&mut self, initial_path: Option<PathBuf>) {
        let Some(path) = initial_path else {
            return;
        };
        let path = path.canonicalize().unwrap_or(path);
        if path.is_dir() {
            self.open_folder(path);
        } else if path.is_file() {
            self.open_file(path);
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
        let revived = self.tabs.sync_missing_documents();
        if !revived.is_empty() {
            self.load_files(revived, None);
        }
        let checks = self.tabs.disk_change_plan();
        if checks.is_empty() {
            return;
        }
        let event_tx = self.runtime.event_tx.clone();
        let wake = self.egui_ctx().clone();
        self.runtime.spawn_blocking(move || {
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
