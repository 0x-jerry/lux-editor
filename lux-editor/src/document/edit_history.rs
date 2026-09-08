//! Undo/redo history: every applied edit is recorded as one transaction
//! (a bundle of sub-edits plus the caret snapshots around it) and replayed
//! back or forward against the buffer.

use super::buffer::DocumentBuffer;
use super::caret_state::CaretSnapshot;

/// One atomic replacement within an edit transaction.
#[derive(Clone, Debug)]
pub struct SubEdit {
    pub start_char: usize,
    pub deleted_text: String,
    pub inserted_text: String,
}

/// A multi-cursor edit bundle: every sub-edit is applied as a single undo step.
#[derive(Clone, Debug)]
pub struct EditTransaction {
    pub edits: Vec<SubEdit>,
    pub before: CaretSnapshot,
    pub after: CaretSnapshot,
}

#[derive(Default)]
pub struct EditHistory {
    undo_stack: Vec<EditTransaction>,
    redo_stack: Vec<EditTransaction>,
}

impl EditHistory {
    const MAX_UNDO_DEPTH: usize = 1000;

    pub fn push(&mut self, transaction: EditTransaction) {
        if is_typed_text_continuation(&self.undo_stack, &transaction) {
            let last = self.undo_stack.last_mut().unwrap();
            last.edits[0]
                .inserted_text
                .push_str(&transaction.edits[0].inserted_text);
            last.after = transaction.after;
            return;
        }
        self.undo_stack.push(transaction);
        if self.undo_stack.len() > Self::MAX_UNDO_DEPTH {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, buffer: &mut DocumentBuffer) -> Option<CaretSnapshot> {
        let transaction = self.undo_stack.pop()?;
        for edit in transaction.edits.iter().rev() {
            apply_replace(
                buffer,
                edit.start_char,
                edit.inserted_text.chars().count(),
                &edit.deleted_text,
            );
        }
        let before = transaction.before.clone();
        self.redo_stack.push(transaction);
        Some(before)
    }

    pub fn redo(&mut self, buffer: &mut DocumentBuffer) -> Option<CaretSnapshot> {
        let transaction = self.redo_stack.pop()?;
        for edit in transaction.edits.iter() {
            apply_replace(
                buffer,
                edit.start_char,
                edit.deleted_text.chars().count(),
                &edit.inserted_text,
            );
        }
        let after = transaction.after.clone();
        self.undo_stack.push(transaction);
        Some(after)
    }
}

/// Merge consecutive zero-deletion inserts by the same cursor into a single
/// undo step (fast typing on one cursor stays one keystroke to undo).
fn is_typed_text_continuation(
    undo_stack: &[EditTransaction],
    transaction: &EditTransaction,
) -> bool {
    let (Some(last), Some(next)) = (undo_stack.last(), transaction.edits.first()) else {
        return false;
    };
    if last.edits.len() != 1 || transaction.edits.len() != 1 {
        return false;
    }
    let prior = &last.edits[0];
    prior.deleted_text.is_empty()
        && next.deleted_text.is_empty()
        && next.start_char == prior.start_char + prior.inserted_text.chars().count()
}

fn apply_replace(buffer: &mut DocumentBuffer, start: usize, remove_len: usize, insert_text: &str) {
    if remove_len > 0 {
        buffer.remove(start..start + remove_len);
    }
    if !insert_text.is_empty() {
        buffer.insert(start, insert_text);
    }
}

#[cfg(test)]
mod tests {
    use super::super::buffer::DocumentBuffer;
    use super::{EditHistory, EditTransaction, SubEdit};

    fn insert(buffer: &mut DocumentBuffer, text: &str) {
        let caret = buffer.text().len_chars();
        buffer.insert(caret, text);
    }

    #[test]
    fn undo_redo_replays_transaction() {
        let mut buffer = DocumentBuffer::new();
        buffer.insert(0, "ab");
        let mut history = EditHistory::default();
        history.push(EditTransaction {
            edits: vec![SubEdit {
                start_char: 2,
                deleted_text: String::new(),
                inserted_text: "c".to_string(),
            }],
            before: Default::default(),
            after: Default::default(),
        });
        buffer.insert(2, "c");

        history.undo(&mut buffer);
        assert_eq!(buffer.text().to_string(), "ab");
        history.redo(&mut buffer);
        assert_eq!(buffer.text().to_string(), "abc");
    }

    #[test]
    fn consecutive_inserts_coalesce_into_one_undo_step() {
        let mut buffer = DocumentBuffer::new();
        let mut history = EditHistory::default();
        for ch in ["a", "b", "c"] {
            let caret = buffer.text().len_chars();
            history.push(EditTransaction {
                edits: vec![SubEdit {
                    start_char: caret,
                    deleted_text: String::new(),
                    inserted_text: ch.to_string(),
                }],
                before: Default::default(),
                after: Default::default(),
            });
            buffer.insert(caret, ch);
        }
        assert_eq!(buffer.text().to_string(), "abc");

        // A single undo removes the whole typed run.
        history.undo(&mut buffer);
        assert_eq!(buffer.text().to_string(), "");
        history.redo(&mut buffer);
        assert_eq!(buffer.text().to_string(), "abc");
    }

    #[test]
    fn non_contiguous_inserts_do_not_coalesce() {
        let mut buffer = DocumentBuffer::new();
        let mut history = EditHistory::default();
        history.push(EditTransaction {
            edits: vec![SubEdit {
                start_char: 0,
                deleted_text: String::new(),
                inserted_text: "a".to_string(),
            }],
            before: Default::default(),
            after: Default::default(),
        });
        buffer.insert(0, "a");
        history.push(EditTransaction {
            edits: vec![SubEdit {
                start_char: 0,
                deleted_text: String::new(),
                inserted_text: "b".to_string(),
            }],
            before: Default::default(),
            after: Default::default(),
        });
        buffer.insert(0, "b");

        history.undo(&mut buffer);
        assert_eq!(buffer.text().to_string(), "a");
        history.undo(&mut buffer);
        assert_eq!(buffer.text().to_string(), "");
    }

    #[test]
    fn multi_edit_undo_restores_all_positions() {
        let mut buffer = DocumentBuffer::new();
        insert(&mut buffer, "one two three");
        let mut history = EditHistory::default();
        // Sub-edits are stored in final-buffer coordinates: with "X" inserted
        // at 0 and "Y" at 8, the latter lands at 9 in the final buffer.
        history.push(EditTransaction {
            edits: vec![
                SubEdit {
                    start_char: 0,
                    deleted_text: String::new(),
                    inserted_text: "X".to_string(),
                },
                SubEdit {
                    start_char: 9,
                    deleted_text: String::new(),
                    inserted_text: "Y".to_string(),
                },
            ],
            before: Default::default(),
            after: Default::default(),
        });
        buffer.insert(8, "Y");
        buffer.insert(0, "X");

        history.undo(&mut buffer);
        assert_eq!(buffer.text().to_string(), "one two three");
        history.redo(&mut buffer);
        assert_eq!(buffer.text().to_string(), "Xone two Ythree");
    }
}
