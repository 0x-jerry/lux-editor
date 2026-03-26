use crate::app::App;
use crate::app::editor::EditTransaction;
use eframe::egui;
use lux_core::Buffer;

impl App {
    pub(super) fn selected_text(&self) -> Option<String> {
        let range = self.caret_state.selection_range()?;
        Some(self.buffer.text().slice(range).to_string())
    }

    pub(super) fn insert_or_replace_selection(&mut self, text: &str, ctx: &egui::Context) -> bool {
        let range = self
            .caret_state
            .selection_range()
            .unwrap_or(self.caret_state.caret_char()..self.caret_state.caret_char());
        self.apply_edit(range.start, range.end, text, ctx)
    }

    pub(super) fn delete_selection(&mut self, ctx: &egui::Context) -> bool {
        let Some(range) = self.caret_state.selection_range() else {
            return false;
        };
        self.apply_edit(range.start, range.end, "", ctx)
    }

    pub(super) fn delete_backward(&mut self, ctx: &egui::Context) -> bool {
        if self.caret_state.selection_range().is_some() {
            return self.delete_selection(ctx);
        }
        let caret = self.caret_state.caret_char();
        if caret == 0 {
            return false;
        }
        self.apply_edit(caret - 1, caret, "", ctx)
    }

    pub(super) fn delete_word_backward(&mut self, ctx: &egui::Context) -> bool {
        if self.caret_state.selection_range().is_some() {
            return self.delete_selection(ctx);
        }
        let caret = self.caret_state.caret_char();
        if caret == 0 {
            return false;
        }
        let start = self.caret_state.previous_word_boundary(&self.buffer);
        self.apply_edit(start, caret, "", ctx)
    }

    pub(super) fn delete_forward(&mut self, ctx: &egui::Context) -> bool {
        if self.caret_state.selection_range().is_some() {
            return self.delete_selection(ctx);
        }
        let caret = self.caret_state.caret_char();
        let total_chars = self.buffer.text().len_chars();
        if caret >= total_chars {
            return false;
        }
        self.apply_edit(caret, caret + 1, "", ctx)
    }

    pub(super) fn delete_word_forward(&mut self, ctx: &egui::Context) -> bool {
        if self.caret_state.selection_range().is_some() {
            return self.delete_selection(ctx);
        }
        let caret = self.caret_state.caret_char();
        let total_chars = self.buffer.text().len_chars();
        if caret >= total_chars {
            return false;
        }
        let end = self.caret_state.next_word_boundary(&self.buffer);
        self.apply_edit(caret, end, "", ctx)
    }

    pub(super) fn apply_edit(
        &mut self,
        start: usize,
        end: usize,
        inserted_text: &str,
        ctx: &egui::Context,
    ) -> bool {
        let total_chars = self.buffer.text().len_chars();
        let start = start.min(total_chars);
        let end = end.min(total_chars).max(start);
        let deleted_text = self.buffer.text().slice(start..end).to_string();
        if deleted_text.is_empty() && inserted_text.is_empty() {
            return false;
        }
        let before = self.caret_state.snapshot();
        if end > start {
            self.buffer.remove(start..end);
        }
        if !inserted_text.is_empty() {
            self.buffer.insert(start, inserted_text);
        }
        let next_caret = start + inserted_text.chars().count();
        self.caret_state
            .set_caret_char(next_caret, &self.buffer, false);
        self.caret_state.clear_selection();
        let after = self.caret_state.snapshot();
        self.edit_history.push(EditTransaction {
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
        self.document_dirty = true;
        self.document_status = Some("Modified".to_string());
        self.update_window_title(ctx);
    }
}
