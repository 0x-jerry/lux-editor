//! The workspace: which folder is open, its lazily loaded tree and watcher, the
//! per-workspace session (open tabs and expanded folders) and the flattened
//! rows the sidebar draws.

use super::Root;
use crate::gpui_ui::tabs::{Tab, TabContent};
use crate::settings::types::WorkspaceSession;
use crate::workspace::{Entry, FileTree, Workspace};
use gpui_kit::*;
use std::path::{Path, PathBuf};

impl Root {
    pub(super) fn open_folder(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = path.canonicalize().unwrap_or(path);
        self.state.workspace.path = Some(path.clone());
        self.state.workspace.file_tree = None;
        self.rescan_workspace();
        self.state.config.add_recent(path.clone(), true);
        self.restore_workspace_session(&path, window, cx);
        cx.notify();
    }

    /// Reopen a workspace's remembered tabs and tree expansion. A workspace seen
    /// for the first time opens with just its root expanded.
    fn restore_workspace_session(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.tree.expanded.clear();
        let Some(session) = self.state.config.workspace_session(path).cloned() else {
            self.state.tree.expanded.insert(path.to_path_buf());
            return;
        };

        self.state.tree.expanded.extend(
            session
                .expanded_dirs
                .iter()
                .filter(|dir| dir.is_dir())
                .cloned(),
        );
        for file in &session.open_files {
            self.open_file(file.clone(), window, cx);
        }
        if let Some(active) = session
            .active_file
            .as_deref()
            .and_then(|active| self.state.tabs.index_of(active))
        {
            self.state.tabs.active = active;
        }
        if session.configuration_open {
            self.open_settings(window, cx);
        }
    }

    /// Snapshot the live tabs and tree expansion into the workspace's session.
    /// The store drops an unchanged snapshot, so this can run on every tick.
    pub(super) fn sync_workspace_session(&mut self) {
        let Some(workspace_path) = self.state.workspace.path.clone() else {
            return;
        };
        let inside = |path: &Path| path.starts_with(&workspace_path);
        let open_files: Vec<PathBuf> = self
            .state
            .tabs
            .tabs
            .iter()
            .filter_map(Tab::text)
            .filter_map(|doc| doc.path.clone())
            .filter(|path| inside(path))
            .collect();
        let active_file = self
            .state
            .tabs
            .active_doc()
            .and_then(|doc| doc.path.clone())
            .filter(|path| inside(path))
            .or_else(|| {
                // A tab from another workspace has focus: keep the remembered
                // file while it is still open instead of erasing it.
                let remembered = self
                    .state
                    .config
                    .workspace_session(&workspace_path)?
                    .active_file
                    .clone()?;
                open_files.contains(&remembered).then_some(remembered)
            });
        let expanded_dirs = self
            .state
            .tree
            .expanded
            .iter()
            .filter(|path| inside(path))
            .cloned()
            .collect();

        self.state
            .config
            .set_workspace_session(WorkspaceSession {
                workspace_path,
                open_files,
                active_file,
                configuration_open: matches!(
                    self.state.tabs.active_tab().map(|tab| &tab.content),
                    Some(TabContent::Settings)
                ),
                expanded_dirs,
            });
    }

    /// Build the workspace's tree and watcher from scratch.
    fn rescan_workspace(&mut self) {
        let Some(path) = self.state.workspace.path.clone() else {
            return;
        };
        let exclude = self.state.config.settings.explorer.exclude.clone();
        self.state.workspace.file_tree = Some(FileTree::new(&path, &exclude));
        self.state.workspace.watcher = Workspace::start_watcher(
            &path,
            &exclude,
            self.state.runtime.event_tx.clone(),
            self.state.runtime.wake.clone(),
        );
    }

    /// Point the open tree at the configured exclusions. A no-op while they are
    /// unchanged, so a settings edit that does not touch them costs nothing; the
    /// watcher is restarted because it captured the old patterns.
    pub(super) fn apply_exclude(&mut self) {
        let Some(path) = self.state.workspace.path.clone() else {
            return;
        };
        let exclude = self.state.config.settings.explorer.exclude.clone();
        let Some(tree) = self.state.workspace.file_tree.as_mut() else {
            return;
        };
        if !tree.set_excluded(&exclude) {
            return;
        }
        self.state.workspace.watcher = Workspace::start_watcher(
            &path,
            &exclude,
            self.state.runtime.event_tx.clone(),
            self.state.runtime.wake.clone(),
        );
    }

    pub(super) fn toggle_dir(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.state.tree.expanded.remove(&path) {
            self.state.tree.expanded.insert(path);
        }
        cx.notify();
    }

    pub(super) fn sidebar_rows(&mut self) -> Vec<Row> {
        let mut rows = Vec::new();
        if let Some(tree) = self.state.workspace.file_tree.as_mut() {
            collect_rows(tree, &self.state.tree.expanded, &mut rows);
        }
        rows
    }
}

pub(super) struct Row {
    pub(super) path: PathBuf,
    pub(super) name: String,
    pub(super) depth: usize,
    pub(super) is_dir: bool,
    pub(super) ignored: bool,
}

fn collect_rows(tree: &mut FileTree, expanded: &std::collections::HashSet<PathBuf>, out: &mut Vec<Row>) {
    let root = tree.root().to_path_buf();
    collect_dir(tree, expanded, &root, 0, out);
}

fn collect_dir(
    tree: &mut FileTree,
    expanded: &std::collections::HashSet<PathBuf>,
    dir: &Path,
    depth: usize,
    out: &mut Vec<Row>,
) {
    for entry in tree.children(dir).iter() {
        let (path, is_dir, ignored) = match entry {
            Entry::File { path, ignored } => (path.clone(), false, *ignored),
            Entry::Directory { path, ignored } => (path.clone(), true, *ignored),
        };
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        // The row has to land before its children, so an expanded directory is
        // followed by its subtree instead of preceded by it.
        let open = is_dir && expanded.contains(&path);
        out.push(Row {
            path: path.clone(),
            name,
            depth,
            is_dir,
            ignored,
        });
        if open {
            collect_dir(tree, expanded, &path, depth + 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    // Deliberately not `use super::*`: gpui exports its own `test` attribute,
    // which would shadow the built-in one below.
    use super::{Row, collect_rows};
    use crate::workspace::FileTree;
    use std::collections::HashSet;

    /// `(name, depth)` per visible row, in the order they are drawn.
    fn layout(rows: &[Row]) -> Vec<(String, usize)> {
        rows.iter()
            .map(|row| (row.name.clone(), row.depth))
            .collect()
    }

    #[test]
    fn an_expanded_directory_is_followed_by_its_children() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path();
        std::fs::create_dir(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "").unwrap();
        std::fs::write(dir.join("top.rs"), "").unwrap();

        let mut tree = FileTree::new(dir, &[]);
        let mut expanded = HashSet::new();
        expanded.insert(dir.join("src"));
        let mut rows = Vec::new();
        collect_rows(&mut tree, &expanded, &mut rows);

        // `src` first, then the child indented under it, then the sibling.
        assert_eq!(
            layout(&rows),
            vec![
                ("src".to_string(), 0),
                ("main.rs".to_string(), 1),
                ("top.rs".to_string(), 0),
            ]
        );
    }
}
