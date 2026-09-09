//! The text content model: the buffer (`buffer`), cursor state
//! (`caret_state`), undo/redo history (`edit_history`), indentation rules
//! (`indent`), the `OpenDocument` aggregate tying them to a file — plus the
//! editing pipeline (`input`), the external formatter (`formatter`) and the
//! text editor UI (`ui`). This is the "text document" content type; tabs
//! (`crate::tabs`) hold it as one variant.

mod buffer;
mod caret_state;
mod edit_history;
mod formatter;
mod indent;
mod input;
#[path = "document.rs"]
mod open_document;
pub(crate) mod ui;

pub(crate) use buffer::DocumentBuffer;
pub(crate) use caret_state::{
    line_column, next_word_boundary, previous_word_boundary, word_char_range,
};
pub(crate) use edit_history::{EditTransaction, SubEdit};
pub(crate) use formatter::run_formatter;
pub(crate) use indent::indentation_for_newline;
pub(crate) use input::{CommandOutcome, EditorCommand, commands_from_event};
pub(crate) use open_document::OpenDocument;
