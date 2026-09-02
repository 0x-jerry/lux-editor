use crate::app::App;
use crate::app::editor::EditTransaction;
use eframe::egui;
use lux_core::Buffer;

impl App {
    pub(super) fn selected_text(&self) -> Option<String> {
        let active_document = self.active_document();
        let range = active_document.caret_state.selection_range()?;
        Some(active_document.buffer.text().slice(range).to_string())
    }

    pub(super) fn copy_selection_to_clipboard(&self, ctx: &egui::Context) -> bool {
        if let Some(selected_text) = self.selected_text() {
            ctx.copy_text(selected_text);
            true
        } else {
            false
        }
    }

    pub(super) fn cut_selection_to_clipboard(&mut self, ctx: &egui::Context) -> bool {
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

    pub(super) fn insert_or_replace_selection(&mut self, text: &str, ctx: &egui::Context) -> bool {
        if let Some(range) = self.active_document().caret_state.selection_range() {
            return self.apply_edit(range.start, range.end, text, ctx);
        }
        let caret = self.active_document().caret_state.caret_char();
        self.apply_edit(caret, caret, text, ctx)
    }

    pub(super) fn delete_selection(&mut self, ctx: &egui::Context) -> bool {
        let Some(range) = self.active_document().caret_state.selection_range() else {
            return false;
        };
        self.apply_edit(range.start, range.end, "", ctx)
    }

    pub(super) fn delete_backward(&mut self, ctx: &egui::Context) -> bool {
        if self
            .active_document()
            .caret_state
            .selection_range()
            .is_some()
        {
            return self.delete_selection(ctx);
        }
        let caret = self.active_document().caret_state.caret_char();
        if caret == 0 {
            return false;
        }
        self.apply_edit(caret - 1, caret, "", ctx)
    }

    pub(super) fn delete_word_backward(&mut self, ctx: &egui::Context) -> bool {
        if self
            .active_document()
            .caret_state
            .selection_range()
            .is_some()
        {
            return self.delete_selection(ctx);
        }
        let active_document = self.active_document();
        let caret = active_document.caret_state.caret_char();
        if caret == 0 {
            return false;
        }
        let start = active_document
            .caret_state
            .previous_word_boundary(&active_document.buffer);
        self.apply_edit(start, caret, "", ctx)
    }

    pub(super) fn delete_forward(&mut self, ctx: &egui::Context) -> bool {
        if self
            .active_document()
            .caret_state
            .selection_range()
            .is_some()
        {
            return self.delete_selection(ctx);
        }
        let active_document = self.active_document();
        let caret = active_document.caret_state.caret_char();
        let total_chars = active_document.buffer.text().len_chars();
        if caret >= total_chars {
            return false;
        }
        self.apply_edit(caret, caret + 1, "", ctx)
    }

    pub(super) fn delete_word_forward(&mut self, ctx: &egui::Context) -> bool {
        if self
            .active_document()
            .caret_state
            .selection_range()
            .is_some()
        {
            return self.delete_selection(ctx);
        }
        let active_document = self.active_document();
        let caret = active_document.caret_state.caret_char();
        let total_chars = active_document.buffer.text().len_chars();
        if caret >= total_chars {
            return false;
        }
        let end = active_document
            .caret_state
            .next_word_boundary(&active_document.buffer);
        self.apply_edit(caret, end, "", ctx)
    }

    pub(super) fn apply_edit(
        &mut self,
        start: usize,
        end: usize,
        inserted_text: &str,
        ctx: &egui::Context,
    ) -> bool {
        let total_chars = self.active_document().buffer.text().len_chars();
        let (mut start, mut end) = if start == end {
            // A collapsed range with an active selection replaces the selection.
            if let Some(range) = self.active_document().caret_state.selection_range() {
                (range.start, range.end)
            } else {
                (start, end)
            }
        } else {
            (start, end)
        };
        start = start.min(total_chars);
        end = end.min(total_chars).max(start);
        let deleted_text = self
            .active_document()
            .buffer
            .text()
            .slice(start..end)
            .to_string();
        if deleted_text.is_empty() && inserted_text.is_empty() {
            return false;
        }
        let before = self.active_document().caret_state.snapshot();
        let next_caret = start + inserted_text.chars().count();
        let after = {
            let active_document = self.active_document_mut();
            if end > start {
                active_document.buffer.remove(start..end);
            }
            if !inserted_text.is_empty() {
                active_document.buffer.insert(start, inserted_text);
            }
            active_document
                .caret_state
                .set_caret_char(next_caret, &active_document.buffer, false);
            // Ensure selection is cleared after any edit, including selection replacement.
            active_document.caret_state.clear_selection();
            active_document.caret_state.clear_preferred_column();
            active_document.caret_state.snapshot()
        };
        self.active_document_mut()
            .edit_history
            .push(EditTransaction {
                start_char: start,
                deleted_text,
                inserted_text: inserted_text.to_string(),
                before,
                after,
            });
        self.mark_document_dirty(ctx);
        true
    }

    pub(super) fn indentation_for_newline(buffer: &Buffer, caret_char: usize) -> String {
        const INDENT: &str = "    ";

        let total_chars = buffer.text().len_chars();
        if total_chars == 0 {
            return "\n".to_string();
        }

        let line_probe = if caret_char == 0 {
            0
        } else {
            caret_char
                .saturating_sub(1)
                .min(total_chars.saturating_sub(1))
        };
        let line_idx = buffer.text().char_to_line(line_probe);
        let line = buffer.text().line(line_idx).to_string();
        let content = line.trim_end_matches(['\n', '\r']);
        let leading = content
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect::<String>();
        let trimmed = content.trim_end();

        if trimmed.ends_with('{') {
            return format!("\n{}{}", leading, INDENT);
        }

        if trimmed.starts_with('}') {
            let dedented = if leading.ends_with('\t') {
                leading.trim_end_matches('\t').to_string()
            } else if leading.ends_with(INDENT) {
                leading.trim_end_matches(INDENT).to_string()
            } else {
                String::new()
            };
            return format!("\n{}", dedented);
        }

        format!("\n{}", leading)
    }

    pub(super) fn mark_document_dirty(&mut self, ctx: &egui::Context) {
        let active_document = self.active_document_mut();
        active_document.document_dirty = true;
        active_document.edit_generation += 1;
        active_document.document_status = Some("Modified".to_string());
        self.update_window_title(ctx);
    }
}
