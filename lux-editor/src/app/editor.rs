use lux_core::Buffer;
use std::ops::Range;

/// A single cursor: a caret position plus an optional selection anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Caret {
    pub caret_char: usize,
    pub anchor_char: Option<usize>,
}

/// Full snapshot of every cursor, used by undo/redo to restore edit state.
#[derive(Clone, Debug, Default)]
pub struct CaretSnapshot {
    pub carets: Vec<Caret>,
    pub active_index: usize,
}

/// Multi-cursor selection state. The active cursor is the one keyboard input
/// and pointer clicks operate on; most movement and edits apply to every
/// cursor at once.
#[derive(Clone, Debug)]
pub struct CaretState {
    carets: Vec<Caret>,
    active_index: usize,
    preferred_columns: Vec<Option<usize>>,
}

impl Default for CaretState {
    fn default() -> Self {
        Self {
            carets: vec![Caret::default()],
            active_index: 0,
            preferred_columns: vec![None],
        }
    }
}

fn range_of(caret: &Caret) -> Option<Range<usize>> {
    let anchor = caret.anchor_char?;
    if anchor == caret.caret_char {
        return None;
    }
    if anchor < caret.caret_char {
        Some(anchor..caret.caret_char)
    } else {
        Some(caret.caret_char..anchor)
    }
}

impl CaretState {
    // ---- snapshots --------------------------------------------------------

    pub fn snapshot(&self) -> CaretSnapshot {
        CaretSnapshot {
            carets: self.carets.clone(),
            active_index: self.active_index,
        }
    }

    pub fn restore(&mut self, snapshot: CaretSnapshot, buffer: &Buffer) {
        let len = buffer.text().len_chars();
        self.carets = snapshot
            .carets
            .iter()
            .map(|caret| Caret {
                caret_char: caret.caret_char.min(len),
                anchor_char: caret.anchor_char.map(|anchor| anchor.min(len)),
            })
            .collect();
        if self.carets.is_empty() {
            self.carets.push(Caret::default());
        }
        self.active_index = snapshot.active_index.min(self.carets.len().saturating_sub(1));
        self.preferred_columns = vec![None; self.carets.len()];
    }

    pub fn reset_to_buffer_end(&mut self, buffer: &Buffer) {
        let len = buffer.text().len_chars();
        self.carets = vec![Caret {
            caret_char: len,
            anchor_char: None,
        }];
        self.active_index = 0;
        self.preferred_columns = vec![None];
    }

    // ---- queries ----------------------------------------------------------

    pub fn len(&self) -> usize {
        self.carets.len()
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }

    pub fn active_caret(&self) -> Caret {
        self.carets[self.active_index.min(self.carets.len().saturating_sub(1))]
    }

    pub fn caret_char_at(&self, index: usize) -> usize {
        self.carets
            .get(index)
            .map_or(0, |caret| caret.caret_char)
    }

    pub fn selection_range(&self) -> Option<Range<usize>> {
        range_of(&self.active_caret())
    }

    pub fn selection_range_at(&self, index: usize) -> Option<Range<usize>> {
        self.carets.get(index).and_then(range_of)
    }

    pub fn selection_ranges(&self) -> Vec<Range<usize>> {
        self.carets.iter().filter_map(range_of).collect()
    }

    /// Every caret character position, indexed by cursor.
    pub fn caret_chars_snapshot(&self) -> Vec<usize> {
        self.carets.iter().map(|caret| caret.caret_char).collect()
    }

    pub fn has_multiple_cursors(&self) -> bool {
        self.carets.len() > 1
    }

    /// The (start, end) replacement target for cursor `index`: its selection
    /// if one exists, otherwise its collapsed caret position.
    pub fn edit_target(&self, index: usize) -> (usize, usize) {
        if let Some(range) = self.selection_range_at(index) {
            (range.start, range.end)
        } else {
            let caret = self.caret_char_at(index);
            (caret, caret)
        }
    }

    // ---- mutation ---------------------------------------------------------

    pub(super) fn set_caret_char_at(
        &mut self,
        index: usize,
        next: usize,
        buffer: &Buffer,
        selecting: bool,
    ) {
        let next = next.min(buffer.text().len_chars());
        let caret = &mut self.carets[index];
        if selecting {
            if caret.anchor_char.is_none() {
                caret.anchor_char = Some(caret.caret_char);
            }
        } else {
            caret.anchor_char = None;
        }
        caret.caret_char = next;
    }

    /// The characters immediately before and after cursor `index`'s caret.
    pub fn neighbor_chars(&self, index: usize, buffer: &Buffer) -> (Option<char>, Option<char>) {
        let caret = self.caret_char_at(index);
        let prev = if caret == 0 {
            None
        } else {
            Some(buffer.text().char(caret - 1))
        };
        let next = if caret >= buffer.text().len_chars() {
            None
        } else {
            Some(buffer.text().char(caret))
        };
        (prev, next)
    }

    /// Move cursor `index` by `delta` characters (used after pairing edits).
    pub(super) fn nudge_caret(&mut self, index: usize, delta: isize, buffer: &Buffer) {
        let current = self.caret_char_at(index);
        let next = (current as isize + delta).max(0) as usize;
        self.set_caret_char_at(index, next.min(buffer.text().len_chars()), buffer, false);
    }

    pub fn set_caret_char(&mut self, next: usize, buffer: &Buffer, selecting: bool) {
        self.set_caret_char_at(self.active_index, next, buffer, selecting);
    }

    /// Move every cursor to the given clamped positions and drop all
    /// selections and preferred columns (used after multi-edit application).
    pub fn set_all_caret_chars(&mut self, positions: &[usize], buffer: &Buffer) {
        let len = buffer.text().len_chars();
        for (index, position) in positions.iter().copied().enumerate() {
            let Some(caret) = self.carets.get_mut(index) else {
                break;
            };
            caret.caret_char = position.min(len);
            caret.anchor_char = None;
        }
        self.preferred_columns = vec![None; self.carets.len()];
    }

    pub fn select_all(&mut self, buffer: &Buffer) {
        let len = buffer.text().len_chars();
        self.carets = vec![Caret {
            caret_char: len,
            anchor_char: Some(0),
        }];
        self.active_index = 0;
        self.preferred_columns = vec![None];
    }

    pub fn select_range(&mut self, start: usize, end: usize, buffer: &Buffer) {
        let total_chars = buffer.text().len_chars();
        self.carets = vec![Caret {
            caret_char: end.min(total_chars),
            anchor_char: Some(start.min(total_chars)),
        }];
        self.active_index = 0;
        self.preferred_columns = vec![None];
    }

    // ---- multi-cursor management ------------------------------------------

    /// Add a cursor at `char_idx`, or activate the existing one there.
    pub fn add_cursor_at(&mut self, char_idx: usize, buffer: &Buffer) {
        let char_idx = char_idx.min(buffer.text().len_chars());
        if let Some(index) = self
            .carets
            .iter()
            .position(|caret| caret.caret_char == char_idx)
        {
            self.active_index = index;
            return;
        }
        self.carets.push(Caret {
            caret_char: char_idx,
            anchor_char: None,
        });
        self.preferred_columns.push(None);
        self.active_index = self.carets.len() - 1;
    }

    pub fn add_cursor_below(&mut self, buffer: &Buffer) {
        let active = self.active_caret();
        let target = line_adjacent_char(buffer, active.caret_char, true);
        self.add_cursor_at(target, buffer);
    }

    pub fn add_cursor_above(&mut self, buffer: &Buffer) {
        let active = self.active_caret();
        let target = line_adjacent_char(buffer, active.caret_char, false);
        self.add_cursor_at(target, buffer);
    }

    /// Collapse back to the single active cursor (e.g. Escape).
    pub fn remove_extra_cursors(&mut self) {
        let active = self.active_caret();
        self.carets = vec![active];
        self.active_index = 0;
        self.preferred_columns = vec![None];
    }

    // ---- movement (applies to every cursor) -------------------------------

    pub fn move_left(&mut self, buffer: &Buffer, selecting: bool) {
        for index in 0..self.carets.len() {
            if !selecting
                && let Some(range) = self.selection_range_at(index)
            {
                self.carets[index].caret_char = range.start;
                self.carets[index].anchor_char = None;
            } else {
                let previous = self.carets[index].caret_char.saturating_sub(1);
                self.set_caret_char_at(index, previous, buffer, selecting);
            }
        }
        self.preferred_columns = vec![None; self.carets.len()];
    }

    pub fn move_right(&mut self, buffer: &Buffer, selecting: bool) {
        for index in 0..self.carets.len() {
            if !selecting && let Some(range) = self.selection_range_at(index) {
                self.carets[index].caret_char = range.end;
                self.carets[index].anchor_char = None;
            } else {
                let next =
                    (self.carets[index].caret_char + 1).min(buffer.text().len_chars());
                self.set_caret_char_at(index, next, buffer, selecting);
            }
        }
        self.preferred_columns = vec![None; self.carets.len()];
    }

    pub fn move_word_left(&mut self, buffer: &Buffer, selecting: bool) {
        for index in 0..self.carets.len() {
            if !selecting && let Some(range) = self.selection_range_at(index) {
                self.carets[index].caret_char = range.start;
                self.carets[index].anchor_char = None;
            } else {
                let target = previous_word_boundary(buffer, self.carets[index].caret_char);
                self.set_caret_char_at(index, target, buffer, selecting);
            }
        }
        self.preferred_columns = vec![None; self.carets.len()];
    }

    pub fn move_word_right(&mut self, buffer: &Buffer, selecting: bool) {
        for index in 0..self.carets.len() {
            if !selecting && let Some(range) = self.selection_range_at(index) {
                self.carets[index].caret_char = range.end;
                self.carets[index].anchor_char = None;
            } else {
                let target = next_word_boundary(buffer, self.carets[index].caret_char);
                self.set_caret_char_at(index, target, buffer, selecting);
            }
        }
        self.preferred_columns = vec![None; self.carets.len()];
    }

    pub fn move_home(&mut self, buffer: &Buffer, selecting: bool) {
        for index in 0..self.carets.len() {
            let line = current_line_index(buffer, self.carets[index].caret_char);
            let start = buffer.text().line_to_char(line);
            self.set_caret_char_at(index, start, buffer, selecting);
        }
        self.preferred_columns = vec![None; self.carets.len()];
    }

    pub fn move_end(&mut self, buffer: &Buffer, selecting: bool) {
        for index in 0..self.carets.len() {
            let line = current_line_index(buffer, self.carets[index].caret_char);
            let end = line_visual_end_char(buffer, line);
            self.set_caret_char_at(index, end, buffer, selecting);
        }
        self.preferred_columns = vec![None; self.carets.len()];
    }

    pub fn move_up(&mut self, buffer: &Buffer, selecting: bool) {
        for index in 0..self.carets.len() {
            self.move_vertically(buffer, index, selecting, false);
        }
    }

    pub fn move_down(&mut self, buffer: &Buffer, selecting: bool) {
        for index in 0..self.carets.len() {
            self.move_vertically(buffer, index, selecting, true);
        }
    }

    fn move_vertically(&mut self, buffer: &Buffer, index: usize, selecting: bool, down: bool) {
        let caret = self.carets[index].caret_char;
        let line = current_line_index(buffer, caret);
        let total_lines = buffer.len_lines();
        if total_lines == 0 {
            return;
        }
        if down && line + 1 >= total_lines {
            self.set_caret_char_at(index, buffer.text().len_chars(), buffer, selecting);
            return;
        }
        if !down && line == 0 {
            self.set_caret_char_at(index, 0, buffer, selecting);
            return;
        }
        let line_start = buffer.text().line_to_char(line);
        let current_column = caret.saturating_sub(line_start);
        let target_column = self.preferred_columns[index].unwrap_or(current_column);
        let target_line = if down { line + 1 } else { line - 1 };
        let target_start = buffer.text().line_to_char(target_line);
        let target_end = line_visual_end_char(buffer, target_line);
        let target = (target_start + target_column).min(target_end);
        self.set_caret_char_at(index, target, buffer, selecting);
        self.preferred_columns[index] = Some(target_column);
    }
}

// ---- history ----------------------------------------------------------------

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
            last.edits[0].inserted_text.push_str(&transaction.edits[0].inserted_text);
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

    pub fn redo(&mut self, buffer: &mut Buffer) -> Option<CaretSnapshot> {
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

fn apply_replace(buffer: &mut Buffer, start: usize, remove_len: usize, insert_text: &str) {
    if remove_len > 0 {
        buffer.remove(start..start + remove_len);
    }
    if !insert_text.is_empty() {
        buffer.insert(start, insert_text);
    }
}

// ---- position helpers -------------------------------------------------------

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

/// Character index on the line below (down = true) or above (down = false),
/// matching the active cursor's column and clamping to the line's visual end.
fn line_adjacent_char(buffer: &Buffer, caret_char: usize, down: bool) -> usize {
    let line = current_line_index(buffer, caret_char);
    let total_lines = buffer.len_lines();
    if down && line + 1 >= total_lines {
        return buffer.text().len_chars();
    }
    if !down && line == 0 {
        return 0;
    }
    let line_start = buffer.text().line_to_char(line);
    let column = caret_char.saturating_sub(line_start);
    let target_line = if down { line + 1 } else { line - 1 };
    let target_start = buffer.text().line_to_char(target_line);
    let target_end = line_visual_end_char(buffer, target_line);
    (target_start + column).min(target_end)
}

pub(super) fn previous_word_boundary(buffer: &Buffer, caret_char: usize) -> usize {
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

pub(super) fn next_word_boundary(buffer: &Buffer, caret_char: usize) -> usize {
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
    use super::{CaretState, EditHistory, EditTransaction, SubEdit, line_column, word_char_range};
    use lux_core::Buffer;

    fn insert(buffer: &mut Buffer, text: &str) {
        let caret = buffer.text().len_chars();
        buffer.insert(caret, text);
    }

    #[test]
    fn selects_all_characters() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "abc");
        let mut caret = CaretState::default();
        caret.select_all(&buffer);
        assert_eq!(caret.selection_range().unwrap_or(0..0), 0..3);
    }

    #[test]
    fn move_word_right_skips_to_next_word_boundary() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "alpha beta");
        let mut caret = CaretState::default();
        caret.move_word_right(&buffer, false);
        assert_eq!(caret.caret_char_at(0), 5);
        caret.move_word_right(&buffer, false);
        assert_eq!(caret.caret_char_at(0), 10);
    }

    #[test]
    fn move_word_left_skips_to_previous_word_boundary() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "alpha beta");
        let mut caret = CaretState::default();
        caret.set_caret_char(buffer.text().len_chars(), &buffer, false);
        caret.move_word_left(&buffer, false);
        assert_eq!(caret.caret_char_at(0), 6);
        caret.move_word_left(&buffer, false);
        assert_eq!(caret.caret_char_at(0), 0);
    }

    #[test]
    fn undo_redo_replays_transaction() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "ab");
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
        let mut buffer = Buffer::new();
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
        let mut buffer = Buffer::new();
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
        let mut buffer = Buffer::new();
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

    #[test]
    fn move_home_and_end_track_line_bounds() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "  hello\nworld");
        let mut caret = CaretState::default();
        caret.set_caret_char(6, &buffer, false); // 'o' on the first line
        caret.move_home(&buffer, false);
        assert_eq!(caret.caret_char_at(0), 0);
        caret.move_end(&buffer, false);
        assert_eq!(caret.caret_char_at(0), 7); // end of "  hello", before the newline
    }

    #[test]
    fn move_down_up_preserve_preferred_column() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "ab\ncdef\ngh");
        let mut caret = CaretState::default();
        caret.set_caret_char(1, &buffer, false); // 'b', column 1 on line 0
        caret.move_down(&buffer, false);
        assert_eq!(caret.caret_char_at(0), 4); // 'd', column 1 on line 1
        caret.move_down(&buffer, false);
        assert_eq!(caret.caret_char_at(0), 9); // 'h', column 1 on short line 2
        caret.move_up(&buffer, false);
        assert_eq!(caret.caret_char_at(0), 4);
    }

    #[test]
    fn move_left_without_select_collapses_selection() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "abc");
        let mut caret = CaretState::default();
        caret.set_caret_char(3, &buffer, true); // select 0..3
        assert_eq!(caret.selection_range(), Some(0..3));
        caret.move_left(&buffer, false);
        assert_eq!(caret.caret_char_at(0), 0);
        assert!(caret.selection_range().is_none());
    }

    #[test]
    fn selection_range_is_normalized_and_length_reported() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "abcde");
        let mut caret = CaretState::default();
        caret.set_caret_char(4, &buffer, true); // anchor 0, caret 4
        assert_eq!(caret.selection_range(), Some(0..4));
        assert_eq!(caret.selection_range().unwrap().len(), 4);
        caret.set_caret_char(0, &buffer, false); // non-selecting move clears
        assert_eq!(caret.selection_range(), None);
    }

    #[test]
    fn add_cursor_dedupes_and_activates() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "abcd");
        let mut caret = CaretState::default();
        caret.set_caret_char(1, &buffer, false);
        caret.add_cursor_at(3, &buffer);
        assert_eq!(caret.len(), 2);
        assert_eq!(caret.active_index(), 1);
        caret.add_cursor_at(3, &buffer); // dedupe: activates existing
        assert_eq!(caret.len(), 2);
        assert_eq!(caret.active_index(), 1);
    }

    #[test]
    fn movement_applies_to_all_cursors() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "abcd efgh");
        let mut caret = CaretState::default();
        caret.set_caret_char(1, &buffer, false);
        caret.add_cursor_at(6, &buffer);
        caret.move_right(&buffer, false);
        assert_eq!(caret.caret_char_at(0), 2);
        assert_eq!(caret.caret_char_at(1), 7);
    }

    #[test]
    fn remove_extra_cursors_keeps_active_only() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "abcd");
        let mut caret = CaretState::default();
        caret.set_caret_char(0, &buffer, false);
        caret.add_cursor_at(2, &buffer);
        caret.add_cursor_at(4, &buffer);
        assert_eq!(caret.len(), 3);
        caret.remove_extra_cursors();
        assert_eq!(caret.len(), 1);
        assert_eq!(caret.caret_char_at(0), 4);
    }

    #[test]
    fn line_column_reports_one_based_positions() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "ab\ncdef");
        assert_eq!(line_column(&buffer, 3), (2, 1)); // 'c' -> line 2, col 1
        assert_eq!(line_column(&buffer, 5), (2, 3)); // 'e' -> line 2, col 3
    }

    #[test]
    fn word_char_range_selects_word_at_middle_and_edges() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "alpha beta");
        assert_eq!(word_char_range(&buffer, 2), Some(0..5)); // middle
        assert_eq!(word_char_range(&buffer, 0), Some(0..5)); // first char
        assert_eq!(word_char_range(&buffer, 4), Some(0..5)); // last char
        assert_eq!(word_char_range(&buffer, 6), Some(6..10)); // second word
    }

    #[test]
    fn word_char_range_handles_punctuation_and_whitespace() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "foo,bar baz");
        assert_eq!(word_char_range(&buffer, 3), Some(3..4));
        assert_eq!(word_char_range(&buffer, 7), None);
        let empty = Buffer::new();
        assert_eq!(word_char_range(&empty, 0), None);
    }

    #[test]
    fn word_char_range_does_not_cross_newlines() {
        let mut buffer = Buffer::new();
        insert(&mut buffer, "foo\nbar");
        assert_eq!(word_char_range(&buffer, 2), Some(0..3));
        assert_eq!(word_char_range(&buffer, 5), Some(4..7));
    }
}