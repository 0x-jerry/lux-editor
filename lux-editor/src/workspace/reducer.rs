//! Workspace reducer: file-system mutations and the watcher-triggered
//! tree refresh.

use crate::app::App;
use crate::events::WorkspaceEvent;

impl App {
    /// File system & workspace: raw mutations of the workspace tree.
    /// Workspace: mutations of the workspace tree and its refresh.
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
                std::fs::rename(old, new).ok();
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

}
