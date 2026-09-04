//! App-specific views built on the component pattern. They speak the app's
//! own event language (`CustomEvent`) directly, so leaves stay thin and the
//! app's reducer stays the single place that mutates state.

pub use about::AboutWindow;
pub use app_view::{AppView, AppViewInput};
pub use command_panel::CommandPanel;
pub use configuration::{ConfigurationView, ConfigurationViewInput};
pub use editor::{EditorView, EditorViewState};
pub use file_tree::{FileTreePanel, FileTreePanelInput};
pub use shell::Shell;

mod about;
mod app_view;
mod command_panel;
mod configuration;
mod document_tabs;
mod editor;
mod file_tree;
mod shell;
mod text_editor;
mod welcome;
