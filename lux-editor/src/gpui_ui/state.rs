//! The gpui-native application state: the runtime, the user configuration, the
//! workspace (folder + lazily loaded tree + watcher), the open tabs and the
//! sidebar's expansion state. Owned by the root view.

use super::runtime::Runtime;
use super::tabs::Tabs;
use crate::settings::Config;
use crate::workspace::Workspace;
use std::collections::HashSet;
use std::path::PathBuf;

pub struct LuxState {
    pub runtime: Runtime,
    pub config: Config,
    pub workspace: Workspace,
    pub tabs: Tabs,
    pub tree: TreeView,
}

/// Which folders the sidebar has expanded.
#[derive(Default)]
pub struct TreeView {
    pub expanded: HashSet<PathBuf>,
}
