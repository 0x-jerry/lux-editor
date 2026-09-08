//! The model behind one open document: the text buffer (`buffer`), cursor
//! state (`caret_state`), undo/redo history (`edit_history`), indentation
//! rules (`indent`) and the `OpenDocument` aggregate tying them to a file.

mod buffer;
mod caret_state;
mod edit_history;
mod indent;
#[path = "document.rs"]
mod open_document;

pub(crate) use buffer::DocumentBuffer;
pub(crate) use caret_state::{
    line_column, next_word_boundary, previous_word_boundary, word_char_range,
};
pub(crate) use edit_history::{EditTransaction, SubEdit};
pub(crate) use indent::indentation_for_newline;
pub(crate) use open_document::OpenDocument;
