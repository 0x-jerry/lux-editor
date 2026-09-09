use crate::document::{
    EditTransaction, OpenDocument, SubEdit, indentation_for_newline, next_word_boundary,
    previous_word_boundary,
};

impl OpenDocument {
    pub(crate) fn selected_text(&self) -> Option<String> {
        let range = self.caret_state.selection_range()?;
        Some(self.buffer.text().slice(range).to_string())
    }

    /// Cut the active cursor's selection, returning the removed text so the
    /// caller can put it on the clipboard; other cursors are left in place.
    pub(crate) fn cut_selection(&mut self) -> Option<String> {
        let range = self.caret_state.selection_range()?;
        let selected_text = self.buffer.text().slice(range.clone()).to_string();
        self.apply_edit(range.start, range.end, "");
        Some(selected_text)
    }

    /// Insert `text` at (or replace the selection of) every cursor.
    pub(crate) fn insert_or_replace_selection(&mut self, text: &str) -> bool {
        let edits = {
            (0..self.caret_state.len())
                .map(|index| self.caret_state.edit_target(index))
                .collect::<Vec<_>>()
        };
        self.apply_multi_edit(edits, text)
    }

    pub(crate) fn delete_backward(&mut self) -> bool {
        let edits = {
            (0..self.caret_state.len())
                .map(|index| {
                    if let Some(range) = self.caret_state.selection_range_at(index) {
                        (range.start, range.end)
                    } else {
                        let caret = self.caret_state.caret_char_at(index);
                        if caret == 0 {
                            (0, 0)
                        } else {
                            (caret - 1, caret)
                        }
                    }
                })
                .collect::<Vec<_>>()
        };
        self.apply_multi_edit(edits, "")
    }

    pub(crate) fn delete_word_backward(&mut self) -> bool {
        let edits = {
            (0..self.caret_state.len())
                .map(|index| {
                    if let Some(range) = self.caret_state.selection_range_at(index) {
                        (range.start, range.end)
                    } else {
                        let caret = self.caret_state.caret_char_at(index);
                        (
                            previous_word_boundary(&self.buffer, caret),
                            caret,
                        )
                    }
                })
                .collect::<Vec<_>>()
        };
        self.apply_multi_edit(edits, "")
    }

    pub(crate) fn delete_forward(&mut self) -> bool {
        let edits = {
            let total_chars = self.buffer.text().len_chars();
            (0..self.caret_state.len())
                .map(|index| {
                    if let Some(range) = self.caret_state.selection_range_at(index) {
                        (range.start, range.end)
                    } else {
                        let caret = self.caret_state.caret_char_at(index);
                        if caret >= total_chars {
                            (total_chars, total_chars)
                        } else {
                            (caret, caret + 1)
                        }
                    }
                })
                .collect::<Vec<_>>()
        };
        self.apply_multi_edit(edits, "")
    }

    pub(crate) fn delete_word_forward(&mut self) -> bool {
        let edits = {
            (0..self.caret_state.len())
                .map(|index| {
                    if let Some(range) = self.caret_state.selection_range_at(index) {
                        (range.start, range.end)
                    } else {
                        let caret = self.caret_state.caret_char_at(index);
                        (caret, next_word_boundary(&self.buffer, caret))
                    }
                })
                .collect::<Vec<_>>()
        };
        self.apply_multi_edit(edits, "")
    }

    /// Enter key: per-cursor indentation.
    pub(crate) fn insert_newline(&mut self) -> bool {
        let (edits, texts) = {
            let cursor_count = self.caret_state.len();
            let mut edits = Vec::with_capacity(cursor_count);
            let mut texts = Vec::with_capacity(cursor_count);
            for index in 0..cursor_count {
                let caret = self.caret_state.caret_char_at(index);
                edits.push((caret, caret));
                texts.push(indentation_for_newline(&self.buffer, caret));
            }
            (edits, texts)
        };
        self.apply_multi_edits(edits, texts)
    }

    /// Apply one replacement to the active cursor's target only; other cursors
    /// keep their position. Whole-buffer replaces (formatter) and active-only
    /// operations (cut) go through here.
    pub(crate) fn apply_edit(&mut self, start: usize, end: usize, inserted_text: &str) -> bool {
        let cursor_count = self.caret_state.len();
        let active_index = self.caret_state.active_index();
        let mut edits = Vec::with_capacity(cursor_count);
        for index in 0..cursor_count {
            if index == active_index {
                edits.push((start, end));
            } else {
                let caret = self.caret_state.caret_char_at(index);
                edits.push((caret, caret));
            }
        }
        let texts = vec![inserted_text.to_string(); cursor_count];
        self.apply_multi_edits(edits, texts)
    }

    /// Apply one replacement with uniform text at every cursor.
    pub(crate) fn apply_multi_edit(
        &mut self,
        edits: Vec<(usize, usize)>,
        inserted_text: &str,
    ) -> bool {
        let texts = vec![inserted_text.to_string(); edits.len()];
        self.apply_multi_edits(edits, texts)
    }

    /// Apply `edits[index]` at cursor `index`, replacing its target with
    /// `texts[index]`. Edits are applied from the highest position down so
    /// earlier indices stay valid; skipped cursors (collapsed target with
    /// empty text) keep their position, shifted by edits below them. All edits
    /// land in a single undo transaction.
    pub(crate) fn apply_multi_edits(
        &mut self,
        edits: Vec<(usize, usize)>,
        texts: Vec<String>,
    ) -> bool {
        let total_chars = self.buffer.text().len_chars();
        let cursor_count = self.caret_state.len();

        // Plan per-cursor replacements, clamped and dropping true no-ops.
        let mut plan: Vec<(usize, usize, usize)> = Vec::new(); // (cursor_index, start, end)
        for (index, (start, end)) in edits.into_iter().enumerate() {
            let start = start.min(total_chars);
            let end = end.min(total_chars).max(start);
            let is_noop = start == end && texts.get(index).is_none_or(|text| text.is_empty());
            if is_noop {
                continue;
            }
            plan.push((index, start, end));
        }
        if plan.is_empty() {
            return false;
        }

        let before = self.caret_state.snapshot();
        let caret_chars_before = self.caret_state.caret_chars_snapshot();

        // Deduplicate identical targets (two cursors landing on the same
        // position must not double-insert).
        plan.sort_by_key(|&(cursor_index, start, end)| (start, end, cursor_index));
        plan.dedup_by(|left, right| left.1 == right.1 && left.2 == right.2);

        // items: (cursor_index, start, end, delta) sorted by start descending.
        let mut items: Vec<(usize, usize, usize, isize)> = plan
            .into_iter()
            .map(|(cursor_index, start, end)| {
                let inserted_len = texts
                    .get(cursor_index)
                    .map_or(0, |text| text.chars().count());
                let delta = inserted_len as isize - (end - start) as isize;
                (cursor_index, start, end, delta)
            })
            .collect();
        items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| right.2.cmp(&left.2)));

        // Undo positions must be in final-buffer coordinates: each edit's
        // start shifts by the net delta of every *lower* edit applied after it.
        let mut final_starts = vec![0usize; items.len()];
        let mut suffix_delta: isize = 0;
        for index in (0..items.len()).rev() {
            final_starts[index] = (items[index].1 as isize + suffix_delta).max(0) as usize;
            suffix_delta += items[index].3;
        }

        let mut edited_position: Vec<(usize, usize)> = Vec::new(); // (cursor_index, new_caret)
        let mut sub_edits: Vec<SubEdit> = Vec::with_capacity(items.len());
        for (index, &(cursor_index, start, end, _)) in items.iter().enumerate() {
            let inserted_text = texts.get(cursor_index).map_or("", |text| text.as_str());
            let deleted_text = self.buffer.text().slice(start..end).to_string();
            if end > start {
                self.buffer.remove(start..end);
            }
            if !inserted_text.is_empty() {
                self.buffer.insert(start, inserted_text);
            }
            let next_caret = start + inserted_text.chars().count();
            edited_position.push((cursor_index, next_caret));
            sub_edits.push(SubEdit {
                start_char: final_starts[index],
                deleted_text,
                inserted_text: inserted_text.to_string(),
            });
        }

        // Final caret positions: edited cursors land after their inserted
        // text; skipped cursors are shifted by inserts below them.
        {
            let mut positions = caret_chars_before.clone();
            for (cursor_index, next_caret) in &edited_position {
                positions[*cursor_index] = *next_caret;
            }
            for index in 0..cursor_count {
                if edited_position.iter().any(|(i, _)| *i == index) {
                    continue;
                }
                let original = caret_chars_before[index];
                let shift: isize = items
                    .iter()
                    .filter(|(_, start, _, delta)| *start <= original && *delta > 0)
                    .map(|(_, _, _, delta)| *delta)
                    .sum();
                positions[index] = (original as isize + shift).max(0) as usize;
            }
            self.caret_state
                .set_all_caret_chars(&positions, &self.buffer);
        }

        sub_edits.sort_by_key(|edit| edit.start_char);
        let after = self.caret_state.snapshot();
        self.edit_history
            .push(EditTransaction {
                edits: sub_edits,
                before,
                after,
            });
        self.mark_dirty();
        true
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.edit_generation += 1;
        // Dirty reflects the buffer against the saved content, not "an edit
        // happened": an edit undone back to that content is clean again.
        let dirty = self.recompute_dirty();
        self.document_status = if dirty {
            Some("Modified".to_string())
        } else {
            None
        };
    }
}