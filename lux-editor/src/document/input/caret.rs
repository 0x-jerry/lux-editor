use crate::document::{OpenDocument, word_char_range};

impl OpenDocument {
    pub(crate) fn set_caret_from_pointer(
        &mut self,
        line_index: usize,
        column: usize,
        selecting: bool,
        add_cursor: bool,
    ) {
        self.edit_area_focused = true;
        let Some(next) = self.pointer_to_char(line_index, column) else {
            self.caret_state.set_caret_char(0, &self.buffer, selecting);
            self.touch_caret_blink();
            return;
        };
        if add_cursor {
            self.caret_state.add_cursor_at(next, &self.buffer);
            self.touch_caret_blink();
            return;
        }
        self.caret_state
            .set_caret_char(next, &self.buffer, selecting);
        self.touch_caret_blink();
    }

    pub(crate) fn select_word_from_pointer(&mut self, line_index: usize, column: usize) {
        self.edit_area_focused = true;
        let Some(char_index) = self.pointer_to_char(line_index, column) else {
            return;
        };
        let Some(word) = word_char_range(&self.buffer, char_index) else {
            return;
        };
        self.caret_state
            .select_range(word.start, word.end, &self.buffer);
        self.touch_caret_blink();
    }

    fn pointer_to_char(&self, line_index: usize, column: usize) -> Option<usize> {
        let total_lines = self.buffer.len_lines();
        if total_lines == 0 {
            return None;
        }
        let line = line_index.min(total_lines.saturating_sub(1));
        let line_start = self.buffer.text().line_to_char(line);
        let line_text = self.buffer.text().line(line).to_string();
        let line_len = line_text.trim_end_matches(['\n', '\r']).chars().count();
        Some(line_start + column.min(line_len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(text: &str) -> OpenDocument {
        let mut document = OpenDocument::new_empty();
        document.buffer.insert(0, text);
        document
    }

    #[test]
    fn selects_the_word_under_the_pointer() {
        let mut document = document("alpha beta");
        document.select_word_from_pointer(0, 2);
        assert_eq!(document.caret_state.selection_range(), Some(0..5));
        assert_eq!(document.caret_state.caret_char_at(0), 5);
        assert_eq!(document.caret_state.len(), 1);
    }

    #[test]
    fn does_not_cross_newlines() {
        let mut document = document("foo\nbar");
        document.select_word_from_pointer(0, 2);
        assert_eq!(document.caret_state.selection_range(), Some(0..3));
        document.select_word_from_pointer(1, 2);
        assert_eq!(document.caret_state.selection_range(), Some(4..7));
    }

    #[test]
    fn whitespace_click_leaves_no_selection_and_keeps_caret() {
        let mut document = document("a b");
        document.select_word_from_pointer(0, 1);
        assert!(document.caret_state.selection_range().is_none());
        assert_eq!(document.caret_state.caret_char_at(0), 0);
        assert_eq!(document.caret_state.len(), 1);
    }

    #[test]
    fn collapses_existing_multi_cursor_state() {
        let mut document = document("alpha beta");
        document.caret_state.set_caret_char(1, &document.buffer, false);
        document.caret_state.add_cursor_at(6, &document.buffer);
        assert_eq!(document.caret_state.len(), 2);
        document.select_word_from_pointer(0, 2);
        assert_eq!(document.caret_state.len(), 1);
        assert_eq!(document.caret_state.selection_range(), Some(0..5));
    }
}