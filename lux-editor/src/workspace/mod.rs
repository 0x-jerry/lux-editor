//! Workspace domain: the open folder, the lazily-loaded file tree that
//! represents it, the watcher that refreshes it and the tree panel UI.
//! Rendering lives in `ui`; the tree model in `tree` is pure data.

mod reducer;
mod state;
mod tree;
pub(crate) mod ui;
mod watcher;

pub(crate) use state::Workspace;
pub(crate) use tree::{Entry, FileTree};
pub(crate) use watcher::watch;
