use lux_core::Buffer;
use std::ops::Range;

#[derive(Clone, Copy, Debug, Default)]
pub struct CaretSnapshot {
    pub caret_char: usize,
    pub anchor_char: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CaretState {
    caret_char: usize,
    anchor_char: Option<usize>,
    preferred_column: Option<usize>,
}

impl CaretState {
    pub fn snapshot(&self) -> CaretSnapshot {
        CaretSnapshot {
            caret_char: self.caret_char,
            anchor_char: self.anchor_char,
        }
    }

    pub fn restore(&mut self, snapshot: CaretSnapshot, buffer: &Buffer) {
        self.caret_char = snapshot.caret_char.min(buffer.text().len_chars());
        self.anchor_char = snapshot
            .anchor_char
            .map(|anchor| anchor.min(buffer.text().len_chars()));
        self.preferred_column = None;
    }

    pub fn reset_to_buffer_end(&mut self, buffer: &Buffer) {
        self.caret_char = buffer.text().len_chars();
        self.anchor_char = None;
        self.preferred_column = None;
    }

    pub fn selection_range(&self) -> Option<Range<usize>> {
        let anchor = self.anchor_char?;
        if anchor == self.caret_char {
            return None;
        }
        if anchor < self.caret_char {
            Some(anchor..self.caret_char)
        } else {
            Some(self.caret_char..anchor)
        }
    }

    pub fn caret_char(&self) -> usize {
        self.caret_char
    }

    pub fn set_caret_char(&mut self, next: usize, buffer: &Buffer, selecting: bool) {
        let next = next.min(buffer.text().len_chars());
        if selecting {
            if self.anchor_char.is_none() {
                self.anchor_char = Some(self.caret_char);
            }
        } else {
            self.anchor_char = None;
        }
        self.caret_char = next;
    }

    pub fn select_all(&mut self, buffer: &Buffer) {
        self.anchor_char = Some(0);
        self.caret_char = buffer.text().len_chars();
        self.preferred_column = None;
    }

    pub fn select_range(&mut self, start: usize, end: usize, buffer: &Buffer) {
        let total_chars = buffer.text().len_chars();
        self.anchor_char = Some(start.min(total_chars));
        self.caret_char = end.min(total_chars);
        self.preferred_column = None;
    }

    pub fn clear_selection(&mut self) {
        self.anchor_char = None;
    }

    pub fn clear_preferred_column(&mut self) {
        self.preferred_column = None;
    }

    pub fn move_left(&mut self, buffer: &Buffer, selecting: bool) {
        if !selecting && let Some(range) = self.selection_range() {
            self.caret_char = range.start;
            self.anchor_char = None;
            self.preferred_column = None;
            return;
        }
        let previous = self.caret_char.saturating_sub(1);
        self.set_caret_char(previous, buffer, selecting);
        self.preferred_column = None;
    }

    pub fn move_right(&mut self, buffer: &Buffer, selecting: bool) {
        if !selecting && let Some(range) = self.selection_range() {
            self.caret_char = range.end;
            self.anchor_char = None;
            self.preferred_column = None;
            return;
        }
        let next = (self.caret_char + 1).min(buffer.text().len_chars());
        self.set_caret_char(next, buffer, selecting);
        self.preferred_column = None;
    }

    pub fn move_word_left(&mut self, buffer: &Buffer, selecting: bool) {
        if !selecting && let Some(range) = self.selection_range() {
            self.caret_char = range.start;
            self.anchor_char = None;
            self.preferred_column = None;
            return;
        }

        let target = self.previous_word_boundary(buffer);
        self.set_caret_char(target, buffer, selecting);
        self.preferred_column = None;
    }

    pub fn move_word_right(&mut self, buffer: &Buffer, selecting: bool) {
        if !selecting && let Some(range) = self.selection_range() {
            self.caret_char = range.end;
            self.anchor_char = None;
            self.preferred_column = None;
            return;
        }

        let target = self.next_word_boundary(buffer);
        self.set_caret_char(target, buffer, selecting);
        self.preferred_column = None;
    }

    pub fn previous_word_boundary(&self, buffer: &Buffer) -> usize {
        previous_word_boundary(buffer, self.caret_char)
    }

    pub fn next_word_boundary(&self, buffer: &Buffer) -> usize {
        next_word_boundary(buffer, self.caret_char)
    }

    pub fn move_home(&mut self, buffer: &Buffer, selecting: bool) {
        let line = current_line_index(buffer, self.caret_char);
        let start = buffer.text().line_to_char(line);
        self.set_caret_char(start, buffer, selecting);
        self.preferred_column = None;
    }

    pub fn move_end(&mut self, buffer: &Buffer, selecting: bool) {
        let line = current_line_index(buffer, self.caret_char);
        let end = line_visual_end_char(buffer, line);
        self.set_caret_char(end, buffer, selecting);
        self.preferred_column = None;
    }

    pub fn move_up(&mut self, buffer: &Buffer, selecting: bool) {
        let line = current_line_index(buffer, self.caret_char);
        if line == 0 {
            self.set_caret_char(0, buffer, selecting);
            return;
        }
        let current_start = buffer.text().line_to_char(line);
        let current_column = self.caret_char.saturating_sub(current_start);
        let target_column = self.preferred_column.unwrap_or(current_column);
        let target_line = line - 1;
        let target_start = buffer.text().line_to_char(target_line);
        let target_end = line_visual_end_char(buffer, target_line);
        let target = (target_start + target_column).min(target_end);
        self.set_caret_char(target, buffer, selecting);
        self.preferred_column = Some(target_column);
    }

    pub fn move_down(&mut self, buffer: &Buffer, selecting: bool) {
        let total_lines = buffer.len_lines();
        if total_lines == 0 {
            self.set_caret_char(0, buffer, selecting);
            return;
        }
        let line = current_line_index(buffer, self.caret_char);
        if line + 1 >= total_lines {
            let end = buffer.text().len_chars();
            self.set_caret_char(end, buffer, selecting);
            return;
        }
        let current_start = buffer.text().line_to_char(line);
        let current_column = self.caret_char.saturating_sub(current_start);
        let target_column = self.preferred_column.unwrap_or(current_column);
        let target_line = line + 1;
        let target_start = buffer.text().line_to_char(target_line);
        let target_end = line_visual_end_char(buffer, target_line);
        let target = (target_start + target_column).min(target_end);
        self.set_caret_char(target, buffer, selecting);
        self.preferred_column = Some(target_column);
    }
}

#[derive(Clone, Debug)]
pub struct EditTransaction {
    pub start_char: usize,
    pub deleted_text: String,
    pub inserted_text: String,
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
        if let Some(last) = self.undo_stack.last_mut()
            && last.deleted_text.is_empty()
            && transaction.deleted_text.is_empty()
            && transaction.start_char == last.start_char + last.inserted_text.chars().count()
        {
            last.inserted_text.push_str(&transaction.inserted_text);
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

    pub fn undo(&mut self, buffer: &mut Buffer) -> Option<CaretSnapshot> {
        let transaction = self.undo_stack.pop()?;
        apply_replace(
            buffer,
            transaction.start_char,
            transaction.inserted_text.chars().count(),
            &transaction.deleted_text,
        );
        let before = transaction.before;
        self.redo_stack.push(transaction);
        Some(before)
    }

    pub fn redo(&mut self, buffer: &mut Buffer) -> Option<CaretSnapshot> {
        let transaction = self.redo_stack.pop()?;
        apply_replace(
            buffer,
            transaction.start_char,
            transaction.deleted_text.chars().count(),
            &transaction.inserted_text,
        );
        let after = transaction.after;
        self.undo_stack.push(transaction);
        Some(after)
    }
}

fn apply_replace(buffer: &mut Buffer, start: usize, remove_len: usize, insert_text: &str) {
    if remove_len > 0 {
        buffer.remove(start..start + remove_len);
    }
    if !insert_text.is_empty() {
        buffer.insert(start, insert_text);
    }
}

pub fn line_column(buffer: &Buffer, caret_char: usize) -> (usize, usize) {
    let line = current_line_index(buffer, caret_char);
    let line_start = buffer.text().line_to_char(line);
    (line + 1, caret_char.saturating_sub(line_start) + 1)
}

fn current_line_index(buffer: &Buffer, caret_char: usize) -> usize {
    let total_chars = buffer.text().len_chars();
    if total_chars == 0 {
        return 0;
    }
    let clamped = caret_char.min(total_chars.saturating_sub(1));
    buffer.text().char_to_line(clamped)
}

fn line_visual_end_char(buffer: &Buffer, line: usize) -> usize {
    let start = buffer.text().line_to_char(line);
    let line_text = buffer.text().line(line).to_string();
    let content_len = line_text.trim_end_matches(['\n', '\r']).chars().count();
    start + content_len
}

fn previous_word_boundary(buffer: &Buffer, caret_char: usize) -> usize {
    if caret_char == 0 {
        return 0;
    }

    let slice = buffer.text().slice(..caret_char);
    let mut index = caret_char;
    while index > 0 && slice.char(index - 1).is_whitespace() {
        index -= 1;
    }
    while index > 0 && is_word_char(slice.char(index - 1)) {
        index -= 1;
    }
    while index > 0 && !slice.char(index - 1).is_whitespace() && !is_word_char(slice.char(index - 1))
    {
        index -= 1;
    }
    index
}

fn next_word_boundary(buffer: &Buffer, caret_char: usize) -> usize {
    let total_chars = buffer.text().len_chars();
    if caret_char >= total_chars {
        return total_chars;
    }

    let slice = buffer.text().slice(caret_char..total_chars);
    let mut offset = 0usize;
    while offset < slice.len_chars() && slice.char(offset).is_whitespace() {
        offset += 1;
    }
    while offset < slice.len_chars() && is_word_char(slice.char(offset)) {
        offset += 1;
    }
    while offset < slice.len_chars()
        && !slice.char(offset).is_whitespace()
        && !is_word_char(slice.char(offset))
    {
        offset += 1;
    }
    caret_char + offset
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

pub(super) fn word_char_range(buffer: &Buffer, char_index: usize) -> Option<Range<usize>> {
    let total_chars = buffer.text().len_chars();
    if total_chars == 0 {
        return None;
    }
    let index = char_index.min(total_chars.saturating_sub(1));
    let ch = buffer.text().char(index);
    if ch == '\n' || ch == '\r' || ch.is_whitespace() {
        return None;
    }
    let word = is_word_char(ch);
    let slice = buffer.text().slice(..total_chars);
    let mut start = index;
    while start > 0 {
        let prev = slice.char(start - 1);
        if (is_word_char(prev)) != word || prev == '\n' || prev == '\r' {
            break;
        }
        start -= 1;
    }
    let mut end = index + 1;
    while end < total_chars {
        let next = slice.char(end);
        if (is_word_char(next)) != word || next == '\n' || next == '\r' {
            break;
        }
        end += 1;
    }
    Some(start..end)
}

#[cfg(test)]
mod tests {
    use super::{CaretState, EditHistory, EditTransaction, line_column, word_char_range};
    use lux_core::Buffer;

    #[test]
    fn selects_all_characters() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "abc");
        let mut caret = CaretState::default();
        caret.select_all(&buffer);
        assert_eq!(caret.selection_range().unwrap_or(0..0), 0..3);
    }

    #[test]
    fn move_word_right_skips_to_next_word_boundary() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "alpha beta");
        let mut caret = CaretState::default();
        caret.move_word_right(&buffer, false);
        assert_eq!(caret.caret_char(), 5);
        caret.move_word_right(&buffer, false);
        assert_eq!(caret.caret_char(), 10);
    }

    #[test]
    fn move_word_left_skips_to_previous_word_boundary() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "alpha beta");
        let mut caret = CaretState::default();
        caret.set_caret_char(buffer.text().len_chars(), &buffer, false);
        caret.move_word_left(&buffer, false);
        assert_eq!(caret.caret_char(), 6);
        caret.move_word_left(&buffer, false);
        assert_eq!(caret.caret_char(), 0);
    }

    #[test]
    fn undo_redo_replays_transaction() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "ab");
        let mut history = EditHistory::default();
        history.push(EditTransaction {
            start_char: 2,
            deleted_text: String::new(),
            inserted_text: "c".to_string(),
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
        let mut buffer = Buffer::new();
        let mut history = EditHistory::default();
        for ch in ["a", "b", "c"] {
            let caret = buffer.text().len_chars();
            history.push(EditTransaction {
                start_char: caret,
                deleted_text: String::new(),
                inserted_text: ch.to_string(),
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
        let mut buffer = Buffer::new();
        let mut history = EditHistory::default();
        history.push(EditTransaction {
            start_char: 0,
            deleted_text: String::new(),
            inserted_text: "a".to_string(),
            before: Default::default(),
            after: Default::default(),
        });
        buffer.insert(0, "a");
        // Edit before the existing text (start_char unchanged) stays a separate step.
        history.push(EditTransaction {
            start_char: 0,
            deleted_text: String::new(),
            inserted_text: "b".to_string(),
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
    fn move_home_and_end_track_line_bounds() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "  hello\nworld");
        let mut caret = CaretState::default();
        caret.set_caret_char(6, &buffer, false); // 'o' on the first line
        caret.move_home(&buffer, false);
        assert_eq!(caret.caret_char(), 0);
        caret.move_end(&buffer, false);
        assert_eq!(caret.caret_char(), 7); // end of "  hello", before the newline
    }

    #[test]
    fn move_down_up_preserve_preferred_column() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "ab\ncdef\ngh");
        let mut caret = CaretState::default();
        caret.set_caret_char(1, &buffer, false); // 'b', column 1 on line 0
        caret.move_down(&buffer, false);
        assert_eq!(caret.caret_char(), 4); // 'd', column 1 on line 1
        caret.move_down(&buffer, false);
        assert_eq!(caret.caret_char(), 9); // 'h', column 1 on short line 2
        caret.move_up(&buffer, false);
        assert_eq!(caret.caret_char(), 4);
    }

    #[test]
    fn move_left_without_select_collapses_selection() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "abc");
        let mut caret = CaretState::default();
        caret.set_caret_char(3, &buffer, true); // select 0..3
        assert_eq!(caret.selection_range(), Some(0..3));
        caret.move_left(&buffer, false);
        assert_eq!(caret.caret_char(), 0);
        assert!(caret.selection_range().is_none());
    }

    #[test]
    fn selection_range_is_normalized_and_length_reported() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "abcde");
        let mut caret = CaretState::default();
        caret.set_caret_char(4, &buffer, true); // anchor 0, caret 4
        assert_eq!(caret.selection_range(), Some(0..4));
        assert_eq!(caret.selection_range().unwrap().len(), 4);
        caret.clear_selection();
        assert_eq!(caret.selection_range(), None);
    }

    #[test]
    fn line_column_reports_one_based_positions() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "ab\ncdef");
        assert_eq!(line_column(&buffer, 3), (2, 1)); // 'c' -> line 2, col 1
        assert_eq!(line_column(&buffer, 5), (2, 3)); // 'e' -> line 2, col 3
    }

    #[test]
    fn word_char_range_selects_word_at_middle_and_edges() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "alpha beta");
        assert_eq!(word_char_range(&buffer, 2), Some(0..5)); // middle
        assert_eq!(word_char_range(&buffer, 0), Some(0..5)); // first char
        assert_eq!(word_char_range(&buffer, 4), Some(0..5)); // last char
        assert_eq!(word_char_range(&buffer, 6), Some(6..10)); // second word
    }

    #[test]
    fn word_char_range_handles_punctuation_and_whitespace() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "foo,bar baz");
        // Punctuation run is its own "word" of non-alphanumerics.
        assert_eq!(word_char_range(&buffer, 3), Some(3..4));
        // Space between words selects nothing.
        assert_eq!(word_char_range(&buffer, 7), None);
        // Empty buffer selects nothing.
        let empty = Buffer::new();
        assert_eq!(word_char_range(&empty, 0), None);
    }

    #[test]
    fn word_char_range_does_not_cross_newlines() {
        let mut buffer = Buffer::new();
        buffer.insert(0, "foo\nbar");
        assert_eq!(word_char_range(&buffer, 2), Some(0..3));
        assert_eq!(word_char_range(&buffer, 5), Some(4..7));
    }
}
