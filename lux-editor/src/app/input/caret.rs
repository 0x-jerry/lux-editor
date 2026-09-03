use super::commands::EditorCommand;
use crate::app::{App, ShellView};
use lux_core::editor::word_char_range;

impl App {
    pub(in crate::app) fn set_caret_from_pointer(
        &mut self,
        line_index: usize,
        column: usize,
        selecting: bool,
        add_cursor: bool,
    ) {
        let Some(next) = self.pointer_to_char(line_index, column) else {
            let active_document = self.active_document_mut();
            active_document
                .caret_state
                .set_caret_char(0, &active_document.buffer, selecting);
            self.documents.touch_caret_blink();
            return;
        };
        if add_cursor {
            let active_document = self.active_document_mut();
            active_document
                .caret_state
                .add_cursor_at(next, &active_document.buffer);
            self.documents.touch_caret_blink();
            return;
        }
        let active_document = self.active_document_mut();
        active_document
            .caret_state
            .set_caret_char(next, &active_document.buffer, selecting);
        self.documents.touch_caret_blink();
    }

    pub(in crate::app) fn select_word_from_pointer(&mut self, line_index: usize, column: usize) {
        let Some(char_index) = self.pointer_to_char(line_index, column) else {
            return;
        };
        let Some(word) = word_char_range(self.buffer(), char_index) else {
            return;
        };
        let active_document = self.active_document_mut();
        active_document
            .caret_state
            .select_range(word.start, word.end, &active_document.buffer);
        self.documents.touch_caret_blink();
    }

    fn pointer_to_char(&self, line_index: usize, column: usize) -> Option<usize> {
        let total_lines = self.buffer().len_lines();
        if total_lines == 0 {
            return None;
        }
        let line = line_index.min(total_lines.saturating_sub(1));
        let line_start = self.buffer().text().line_to_char(line);
        let line_text = self.buffer().text().line(line).to_string();
        let line_len = line_text.trim_end_matches(['\n', '\r']).chars().count();
        Some(line_start + column.min(line_len))
    }

    pub(super) fn should_ignore_editor_command(
        &self,
        command: &EditorCommand,
        ctx: &eframe::egui::Context,
    ) -> bool {
        if matches!(command, EditorCommand::ToggleCommandPanel) {
            return false;
        }

        self.chrome.command_panel.open()
            || self.chrome.shell.shell_view() != ShellView::Editor
            || ctx.egui_wants_keyboard_input()
    }
}