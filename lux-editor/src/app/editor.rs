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

    pub fn clear_selection(&mut self) {
        self.anchor_char = None;
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

    pub fn selection_len(&self) -> usize {
        self.selection_range()
            .map(|range| range.end - range.start)
            .unwrap_or(0)
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
    pub fn push(&mut self, transaction: EditTransaction) {
        self.undo_stack.push(transaction);
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

    let chars = buffer
        .text()
        .slice(..caret_char)
        .to_string()
        .chars()
        .collect::<Vec<_>>();
    let mut index = chars.len();

    while index > 0 && chars[index - 1].is_whitespace() {
        index -= 1;
    }
    while index > 0 && is_word_char(chars[index - 1]) {
        index -= 1;
    }
    while index > 0 && !chars[index - 1].is_whitespace() && !is_word_char(chars[index - 1]) {
        index -= 1;
    }

    index
}

fn next_word_boundary(buffer: &Buffer, caret_char: usize) -> usize {
    let total_chars = buffer.text().len_chars();
    if caret_char >= total_chars {
        return total_chars;
    }

    let chars = buffer
        .text()
        .slice(caret_char..total_chars)
        .to_string()
        .chars()
        .collect::<Vec<_>>();
    let mut offset = 0usize;

    while offset < chars.len() && chars[offset].is_whitespace() {
        offset += 1;
    }
    while offset < chars.len() && is_word_char(chars[offset]) {
        offset += 1;
    }
    while offset < chars.len() && !chars[offset].is_whitespace() && !is_word_char(chars[offset]) {
        offset += 1;
    }

    caret_char + offset
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::{CaretState, EditHistory, EditTransaction};
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
}
