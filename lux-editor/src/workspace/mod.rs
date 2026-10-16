//! Workspace domain: the open folder, the lazily-loaded file tree that
//! represents it and the watcher that refreshes it. The tree model in `tree`
//! is pure data; the gpui tree view renders it.

mod state;
mod tree;
mod watcher;

pub(crate) use state::Workspace;
pub(crate) use tree::{Entry, FileTree};
pub(crate) use watcher::watch;
