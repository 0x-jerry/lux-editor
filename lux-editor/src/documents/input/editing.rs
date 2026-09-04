use crate::app::App;
use eframe::egui;
use lux_core::editor::{
    EditTransaction, INDENT, SubEdit, indentation_for_newline, leading_indent, next_word_boundary,
    previous_word_boundary,
};
use lux_core::pairing::{PairingAction, action_for, closing_pair, matched_pair_around};

impl App {
    pub(crate) fn selected_text(&self) -> Option<String> {
        let active_document = self.active_document();
        let range = active_document.caret_state.selection_range()?;
        Some(active_document.buffer.text().slice(range).to_string())
    }

    pub(crate) fn copy_selection_to_clipboard(&self, ctx: &egui::Context) -> bool {
        if let Some(selected_text) = self.selected_text() {
            ctx.copy_text(selected_text);
            true
        } else {
            false
        }
    }

    /// Cut the active cursor's selection; other cursors are left in place.
    pub(crate) fn cut_selection_to_clipboard(&mut self, ctx: &egui::Context) -> bool {
        let Some(range) = self.active_document().caret_state.selection_range() else {
            return false;
        };
        let selected_text = self
            .active_document()
            .buffer
            .text()
            .slice(range.clone())
            .to_string();
        ctx.copy_text(selected_text);
        self.apply_edit(range.start, range.end, "", ctx)
    }

    /// Insert `text` at (or replace the selection of) every cursor.
    pub(crate) fn insert_or_replace_selection(&mut self, text: &str, ctx: &egui::Context) -> bool {
        let edits = {
            let active_document = self.active_document();
            (0..active_document.caret_state.len())
                .map(|index| active_document.caret_state.edit_target(index))
                .collect::<Vec<_>>()
        };
        self.apply_multi_edit(edits, text, ctx)
    }

    /// Typed single-char insert with smart pairing: auto-close openers, skip
    /// over closing partners, and no pairing behavior over selections.
    pub(crate) fn insert_text_with_pairing(&mut self, text: &str, ctx: &egui::Context) -> bool {
        let enabled = self.settings.editor_config.settings.behavior.smart_pairing;
        if !enabled || text.chars().count() != 1 {
            return self.insert_or_replace_selection(text, ctx);
        }
        let ch = text.chars().next().unwrap();

        let (edits, texts, auto_close, skip) = {
            let active_document = self.active_document();
            let cursor_count = active_document.caret_state.len();
            let mut edits = Vec::with_capacity(cursor_count);
            let mut texts = Vec::with_capacity(cursor_count);
            let mut auto_close = Vec::new();
            let mut skip = Vec::new();
            for index in 0..cursor_count {
                let (start, end) = active_document.caret_state.edit_target(index);
                let collapsed = start == end;
                let (prev, next) = if collapsed {
                    active_document
                        .caret_state
                        .neighbor_chars(index, &active_document.buffer)
                } else {
                    (None, None)
                };
                let action = if collapsed {
                    action_for(ch, prev, next)
                } else {
                    PairingAction::Plain
                };
                match action {
                    PairingAction::AutoClose => {
                        let closer = closing_pair(ch).unwrap();
                        edits.push((start, end));
                        texts.push(format!("{ch}{closer}"));
                        auto_close.push(index);
                    }
                    PairingAction::Skip => {
                        edits.push((start, end));
                        texts.push(String::new());
                        skip.push(index);
                    }
                    PairingAction::Plain => {
                        edits.push((start, end));
                        texts.push(text.to_string());
                    }
                }
            }
            (edits, texts, auto_close, skip)
        };

        let applied = self.apply_multi_edits(edits, texts, ctx);
        let mut nudged = false;
        {
            let active_document = self.active_document_mut();
            // Auto-close inserted both chars; settle the caret between them.
            for index in auto_close {
                active_document
                    .caret_state
                    .nudge_caret(index, -1, &active_document.buffer);
                nudged = true;
            }
            // Skip-over: move past the already-present closing char.
            for index in skip {
                active_document
                    .caret_state
                    .nudge_caret(index, 1, &active_document.buffer);
                nudged = true;
            }
        }
        applied || nudged
    }

    pub(crate) fn delete_backward(&mut self, ctx: &egui::Context) -> bool {
        let smart_pairing = self.settings.editor_config.settings.behavior.smart_pairing;
        let edits = {
            let active_document = self.active_document();
            (0..active_document.caret_state.len())
                .map(|index| {
                    if let Some(range) = active_document.caret_state.selection_range_at(index) {
                        (range.start, range.end)
                    } else {
                        let caret = active_document.caret_state.caret_char_at(index);
                        if caret == 0 {
                            (0, 0)
                        } else if smart_pairing {
                            let (prev, next) = active_document
                                .caret_state
                                .neighbor_chars(index, &active_document.buffer);
                            if matched_pair_around(prev, next) {
                                (caret - 1, caret + 1)
                            } else {
                                (caret - 1, caret)
                            }
                        } else {
                            (caret - 1, caret)
                        }
                    }
                })
                .collect::<Vec<_>>()
        };
        self.apply_multi_edit(edits, "", ctx)
    }

    pub(crate) fn delete_word_backward(&mut self, ctx: &egui::Context) -> bool {
        let edits = {
            let active_document = self.active_document();
            (0..active_document.caret_state.len())
                .map(|index| {
                    if let Some(range) = active_document.caret_state.selection_range_at(index) {
                        (range.start, range.end)
                    } else {
                        let caret = active_document.caret_state.caret_char_at(index);
                        (
                            previous_word_boundary(&active_document.buffer, caret),
                            caret,
                        )
                    }
                })
                .collect::<Vec<_>>()
        };
        self.apply_multi_edit(edits, "", ctx)
    }

    pub(crate) fn delete_forward(&mut self, ctx: &egui::Context) -> bool {
        let edits = {
            let active_document = self.active_document();
            let total_chars = active_document.buffer.text().len_chars();
            (0..active_document.caret_state.len())
                .map(|index| {
                    if let Some(range) = active_document.caret_state.selection_range_at(index) {
                        (range.start, range.end)
                    } else {
                        let caret = active_document.caret_state.caret_char_at(index);
                        if caret >= total_chars {
                            (total_chars, total_chars)
                        } else {
                            (caret, caret + 1)
                        }
                    }
                })
                .collect::<Vec<_>>()
        };
        self.apply_multi_edit(edits, "", ctx)
    }

    pub(crate) fn delete_word_forward(&mut self, ctx: &egui::Context) -> bool {
        let edits = {
            let active_document = self.active_document();
            (0..active_document.caret_state.len())
                .map(|index| {
                    if let Some(range) = active_document.caret_state.selection_range_at(index) {
                        (range.start, range.end)
                    } else {
                        let caret = active_document.caret_state.caret_char_at(index);
                        (caret, next_word_boundary(&active_document.buffer, caret))
                    }
                })
                .collect::<Vec<_>>()
        };
        self.apply_multi_edit(edits, "", ctx)
    }

    /// Enter key: per-cursor indentation, with smart newlines that open a
    /// blank line inside an empty paired region (`(|)` → two indented lines).
    pub(crate) fn insert_newline(&mut self, ctx: &egui::Context) -> bool {
        let smart_pairing = self.settings.editor_config.settings.behavior.smart_pairing;
        let (edits, texts, smart_mid) = {
            let active_document = self.active_document();
            let cursor_count = active_document.caret_state.len();
            let mut edits = Vec::with_capacity(cursor_count);
            let mut texts = Vec::with_capacity(cursor_count);
            let mut smart_mid = Vec::new();
            for index in 0..cursor_count {
                let caret = active_document.caret_state.caret_char_at(index);
                edits.push((caret, caret));
                let (prev, next) = active_document
                    .caret_state
                    .neighbor_chars(index, &active_document.buffer);
                if smart_pairing && matched_pair_around(prev, next) {
                    let leading = leading_indent(&active_document.buffer, caret);
                    let inner = format!("{leading}{INDENT}");
                    let text = format!("\n{inner}\n{leading}");
                    smart_mid.push((index, caret + 1 + inner.chars().count()));
                    texts.push(text);
                } else {
                    texts.push(indentation_for_newline(&active_document.buffer, caret));
                }
            }
            (edits, texts, smart_mid)
        };

        let applied = self.apply_multi_edits(edits, texts, ctx);
        let mut positioned = false;
        if !smart_mid.is_empty() {
            let active_document = self.active_document_mut();
            for (index, desired) in smart_mid {
                active_document.caret_state.set_caret_char_at(
                    index,
                    desired,
                    &active_document.buffer,
                    false,
                );
                positioned = true;
            }
        }
        applied || positioned
    }

    /// Apply one replacement to the active cursor's target only; other cursors
    /// keep their position. Whole-buffer replaces (formatter) and active-only
    /// operations (cut) go through here.
    pub(crate) fn apply_edit(
        &mut self,
        start: usize,
        end: usize,
        inserted_text: &str,
        ctx: &egui::Context,
    ) -> bool {
        let active_document = self.active_document();
        let cursor_count = active_document.caret_state.len();
        let active_index = active_document.caret_state.active_index();
        let mut edits = Vec::with_capacity(cursor_count);
        for index in 0..cursor_count {
            if index == active_index {
                edits.push((start, end));
            } else {
                let caret = active_document.caret_state.caret_char_at(index);
                edits.push((caret, caret));
            }
        }
        let texts = vec![inserted_text.to_string(); cursor_count];
        self.apply_multi_edits(edits, texts, ctx)
    }

    /// Apply one replacement with uniform text at every cursor.
    pub(crate) fn apply_multi_edit(
        &mut self,
        edits: Vec<(usize, usize)>,
        inserted_text: &str,
        ctx: &egui::Context,
    ) -> bool {
        let texts = vec![inserted_text.to_string(); edits.len()];
        self.apply_multi_edits(edits, texts, ctx)
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
        ctx: &egui::Context,
    ) -> bool {
        let total_chars = self.active_document().buffer.text().len_chars();
        let cursor_count = self.active_document().caret_state.len();

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

        let before = self.active_document().caret_state.snapshot();
        let caret_chars_before = self.active_document().caret_state.caret_chars_snapshot();

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
        {
            let active_document = self.active_document_mut();
            for (index, &(cursor_index, start, end, _)) in items.iter().enumerate() {
                let inserted_text = texts.get(cursor_index).map_or("", |text| text.as_str());
                let deleted_text = active_document.buffer.text().slice(start..end).to_string();
                if end > start {
                    active_document.buffer.remove(start..end);
                }
                if !inserted_text.is_empty() {
                    active_document.buffer.insert(start, inserted_text);
                }
                let next_caret = start + inserted_text.chars().count();
                edited_position.push((cursor_index, next_caret));
                sub_edits.push(SubEdit {
                    start_char: final_starts[index],
                    deleted_text,
                    inserted_text: inserted_text.to_string(),
                });
            }
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
            let active_document = self.active_document_mut();
            active_document
                .caret_state
                .set_all_caret_chars(&positions, &active_document.buffer);
        }

        sub_edits.sort_by_key(|edit| edit.start_char);
        let after = self.active_document().caret_state.snapshot();
        self.active_document_mut()
            .edit_history
            .push(EditTransaction {
                edits: sub_edits,
                before,
                after,
            });
        self.mark_document_dirty(ctx);
        true
    }

    pub(crate) fn mark_document_dirty(&mut self, ctx: &egui::Context) {
        let active_document = self.active_document_mut();
        active_document.document_dirty = true;
        active_document.edit_generation += 1;
        active_document.document_status = Some("Modified".to_string());
        self.update_window_title(ctx);
    }
}
