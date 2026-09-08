//! Documents domain: the open files and their tabs. Owns the tab list and
//! per-document state (`state`), the editing/caret input pipeline (`input`),
//! the external formatter (`formatter`) and the UI that renders and paints
//! them (`ui`, `tabs`). The model behind each document lives in
//! [`crate::document`].

mod formatter;
mod input;
mod state;
mod tabs;
pub(crate) mod ui;

pub(crate) use formatter::run_formatter;
pub(crate) use input::{EditorCommand, commands_from_event};
pub(crate) use state::{Documents, openable_tab, tab_with_path};
pub use tabs::DocumentTab;
pub use ui::{EditorView, EditorViewState};
