//! Chrome views built on the component pattern. They speak the app's event
//! language directly, so leaves stay thin and the app's actions (`app/actions`)
//! stay the only place that mutates state across domains.

pub(crate) mod widgets;

mod about;
mod app_view;
mod command_panel;
mod file_binary;
pub(crate) mod file_image;
mod file_missing;
pub(crate) mod frame_input;
mod shell;
pub(crate) mod welcome;
mod workspace_start;

pub use about::AboutWindow;
pub use app_view::{AppView, AppViewInput};
pub use command_panel::CommandPanel;
pub use file_binary::{FileBinaryInput, FileBinaryView};
pub use file_image::{FileImageInput, FileImageView, is_image_path};
pub use file_missing::{FileMissingInput, FileMissingView};
pub use frame_input::{CaretInput, DocumentInput, SidebarInput, TabsInput, WorkspaceInput};
pub use shell::Shell;
pub use workspace_start::{WorkspaceStartInput, WorkspaceStartView};
