//! The app's event bus: background workers (file watchers, timers) send a
//! [`CustomEvent`] over the runtime channel and wake the UI, which drains them
//! in the gpui root's pump. One variant per domain, so a new event has to pick
//! a domain:
//!
//! - [`WorkspaceEvent`] — workspace-tree changes
//! - [`AppEvent`] — app-global state (config refresh)

/// Workspace: changes to the open workspace tree.
#[derive(Debug)]
pub enum WorkspaceEvent {
    /// A watched file changed; the tree and the open documents need re-checking.
    FileChange,
}

/// App-level state: the user's settings file changed on disk.
#[derive(Debug)]
pub enum AppEvent {
    /// The settings file was written by someone else; reload it.
    ConfigChange,
}

/// The event-bus envelope: one variant per domain.
#[derive(Debug)]
pub enum CustomEvent {
    Workspace(WorkspaceEvent),
    App(AppEvent),
}
