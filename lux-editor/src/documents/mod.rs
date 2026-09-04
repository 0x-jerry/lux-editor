//! Documents domain: the open files. Owns the tab list and per-document
//! editor state (`state`, `document`), the editing/caret input pipeline
//! (`input`), the external formatter (`formatter`) and the UI that renders
//! and paints them (`ui`, `tabs`).

mod document;
mod formatter;
mod input;
mod reducer;
mod state;
mod tabs;
pub(crate) mod ui;

pub(crate) use document::OpenDocument;
pub(crate) use input::EditorCommand;
pub(crate) use state::Documents;
pub use tabs::DocumentTab;
pub use ui::{EditorView, EditorViewState};
