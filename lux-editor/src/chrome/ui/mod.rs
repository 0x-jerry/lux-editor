//! Chrome views built on the component pattern. They speak the app's event
//! language directly, so leaves stay thin and the reducers stay the only
//! place that mutates state.

pub(crate) mod widgets;

mod about;
mod app_view;
mod command_panel;
mod file_binary;
mod file_missing;
mod shell;
pub(crate) mod welcome;
mod workspace_start;

pub use about::AboutWindow;
pub use app_view::{AppView, AppViewInput};
pub use command_panel::CommandPanel;
pub use file_binary::{FileBinaryInput, FileBinaryView};
pub use file_missing::{FileMissingInput, FileMissingView};
pub use shell::Shell;
pub use workspace_start::{WorkspaceStartInput, WorkspaceStartView};
