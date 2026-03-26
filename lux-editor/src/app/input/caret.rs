use super::commands::EditorCommand;
use crate::app::editor::line_column;
use crate::app::{App, ShellView};

impl App {
    const CARET_BLINK_PERIOD: std::time::Duration = std::time::Duration::from_millis(1000);

    pub(in crate::app) fn reset_editor_state(&mut self) {
        self.caret_state.reset_to_buffer_end(&self.buffer);
        self.edit_history.clear();
        self.touch_caret_blink();
    }

    pub(in crate::app) fn caret_position(&self) -> (usize, usize) {
        line_column(&self.buffer, self.caret_state.caret_char())
    }

    pub(in crate::app) fn selection_len(&self) -> usize {
        self.caret_state.selection_len()
    }

    pub(in crate::app) fn set_caret_from_pointer(
        &mut self,
        line_index: usize,
        column: usize,
        selecting: bool,
    ) {
        let total_lines = self.buffer.len_lines();
        if total_lines == 0 {
            self.caret_state.set_caret_char(0, &self.buffer, selecting);
            self.touch_caret_blink();
            return;
        }
        let line = line_index.min(total_lines.saturating_sub(1));
        let line_start = self.buffer.text().line_to_char(line);
        let line_text = self.buffer.text().line(line).to_string();
        let line_len = line_text.trim_end_matches(['\n', '\r']).chars().count();
        let next = line_start + column.min(line_len);
        self.caret_state
            .set_caret_char(next, &self.buffer, selecting);
        self.touch_caret_blink();
    }

    pub(in crate::app) fn caret_blink_visible(&self) -> bool {
        self.caret_blink_anchor.elapsed().as_millis() % Self::CARET_BLINK_PERIOD.as_millis()
            < (Self::CARET_BLINK_PERIOD.as_millis() / 2)
    }

    pub(super) fn touch_caret_blink(&mut self) {
        self.caret_blink_anchor = std::time::Instant::now();
    }

    pub(super) fn should_ignore_editor_command(
        &self,
        command: &EditorCommand,
        ctx: &eframe::egui::Context,
    ) -> bool {
        if matches!(command, EditorCommand::ToggleCommandPanel) {
            return false;
        }

        self.command_panel_open()
            || self.shell_view != ShellView::Editor
            || ctx.wants_keyboard_input()
    }
}
