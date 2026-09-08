//! Workspace domain: the open folder, the lazily-loaded file tree that
//! represents it, the watcher that refreshes it and the tree panel UI.
//! Rendering lives in `file_tree_panel`; the tree model in `tree` is pure data.

pub(crate) mod file_tree_panel;
mod state;
mod tree;
mod watcher;

pub(crate) use state::Workspace;
pub(crate) use tree::{Entry, FileTree};
pub(crate) use watcher::watch;
