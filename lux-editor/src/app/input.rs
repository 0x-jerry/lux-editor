use super::App;
use super::editor::{EditTransaction, line_column};
use eframe::egui;
use lux_core::Buffer;

enum EditorCommand {
    InsertText(String),
    Paste(String),
    InsertNewline,
    InsertTab,
    Backspace,
    Delete,
    MoveLeft { selecting: bool },
    MoveRight { selecting: bool },
    MoveUp { selecting: bool },
    MoveDown { selecting: bool },
    MoveHome { selecting: bool },
    MoveEnd { selecting: bool },
    SelectAll,
    Copy,
    Cut,
    Undo,
    Redo,
}

impl App {
    const CARET_BLINK_PERIOD: std::time::Duration = std::time::Duration::from_millis(1000);

    pub(super) fn reset_editor_state(&mut self) {
        self.caret_state.reset_to_buffer_end(&self.buffer);
        self.edit_history.clear();
        self.touch_caret_blink();
    }

    pub(super) fn caret_position(&self) -> (usize, usize) {
        line_column(&self.buffer, self.caret_state.caret_char())
    }

    pub(super) fn selection_len(&self) -> usize {
        self.caret_state.selection_len()
    }

    pub(super) fn set_caret_from_pointer(
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

    pub(super) fn caret_blink_visible(&self) -> bool {
        self.caret_blink_anchor.elapsed().as_millis() % Self::CARET_BLINK_PERIOD.as_millis()
            < (Self::CARET_BLINK_PERIOD.as_millis() / 2)
    }

    fn touch_caret_blink(&mut self) {
        self.caret_blink_anchor = std::time::Instant::now();
    }

    pub(super) fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }

        let mut changed = false;
        let events = ctx.input(|input| input.events.clone());
        for event in events {
            for command in Self::commands_from_event(event) {
                if self.execute_command(command, ctx) {
                    changed = true;
                }
            }
        }

        if changed {
            self.refresh_language_intelligence();
        }
    }

    fn commands_from_event(event: egui::Event) -> Vec<EditorCommand> {
        match event {
            egui::Event::Text(text) => {
                if text.starts_with(|c: char| c.is_ascii_control()) {
                    Vec::new()
                } else {
                    vec![EditorCommand::InsertText(text)]
                }
            }
            egui::Event::Paste(text) => vec![EditorCommand::Paste(text)],
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                let shortcut = modifiers.command || modifiers.ctrl;
                let selecting = modifiers.shift;
                if shortcut && !modifiers.alt {
                    return match key {
                        egui::Key::A => vec![EditorCommand::SelectAll],
                        egui::Key::C => vec![EditorCommand::Copy],
                        egui::Key::X => vec![EditorCommand::Cut],
                        egui::Key::Z if modifiers.shift => vec![EditorCommand::Redo],
                        egui::Key::Z => vec![EditorCommand::Undo],
                        egui::Key::Y => vec![EditorCommand::Redo],
                        _ => Vec::new(),
                    };
                }
                if modifiers.alt {
                    return Vec::new();
                }
                match key {
                    egui::Key::Enter => vec![EditorCommand::InsertNewline],
                    egui::Key::Tab => vec![EditorCommand::InsertTab],
                    egui::Key::Backspace => vec![EditorCommand::Backspace],
                    egui::Key::Delete => vec![EditorCommand::Delete],
                    egui::Key::ArrowLeft => vec![EditorCommand::MoveLeft { selecting }],
                    egui::Key::ArrowRight => vec![EditorCommand::MoveRight { selecting }],
                    egui::Key::ArrowUp => vec![EditorCommand::MoveUp { selecting }],
                    egui::Key::ArrowDown => vec![EditorCommand::MoveDown { selecting }],
                    egui::Key::Home => vec![EditorCommand::MoveHome { selecting }],
                    egui::Key::End => vec![EditorCommand::MoveEnd { selecting }],
                    _ => Vec::new(),
                }
            }
            _ => Vec::new(),
        }
    }

    fn execute_command(&mut self, command: EditorCommand, ctx: &egui::Context) -> bool {
        if !matches!(&command, EditorCommand::Copy) {
            self.touch_caret_blink();
        }
        match command {
            EditorCommand::InsertText(text) | EditorCommand::Paste(text) => {
                self.insert_or_replace_selection(&text)
            }
            EditorCommand::InsertNewline => {
                let indentation =
                    Self::indentation_for_newline(&self.buffer, self.caret_state.caret_char());
                self.insert_or_replace_selection(&indentation)
            }
            EditorCommand::InsertTab => self.insert_or_replace_selection("    "),
            EditorCommand::Backspace => self.delete_backward(),
            EditorCommand::Delete => self.delete_forward(),
            EditorCommand::MoveLeft { selecting } => {
                self.caret_state.move_left(&self.buffer, selecting);
                false
            }
            EditorCommand::MoveRight { selecting } => {
                self.caret_state.move_right(&self.buffer, selecting);
                false
            }
            EditorCommand::MoveUp { selecting } => {
                self.caret_state.move_up(&self.buffer, selecting);
                false
            }
            EditorCommand::MoveDown { selecting } => {
                self.caret_state.move_down(&self.buffer, selecting);
                false
            }
            EditorCommand::MoveHome { selecting } => {
                self.caret_state.move_home(&self.buffer, selecting);
                false
            }
            EditorCommand::MoveEnd { selecting } => {
                self.caret_state.move_end(&self.buffer, selecting);
                false
            }
            EditorCommand::SelectAll => {
                self.caret_state.select_all(&self.buffer);
                false
            }
            EditorCommand::Copy => {
                if let Some(selected_text) = self.selected_text() {
                    ctx.copy_text(selected_text);
                }
                false
            }
            EditorCommand::Cut => {
                if let Some(selected_text) = self.selected_text() {
                    ctx.copy_text(selected_text);
                    self.delete_selection()
                } else {
                    false
                }
            }
            EditorCommand::Undo => {
                if let Some(snapshot) = self.edit_history.undo(&mut self.buffer) {
                    self.caret_state.restore(snapshot, &self.buffer);
                    true
                } else {
                    false
                }
            }
            EditorCommand::Redo => {
                if let Some(snapshot) = self.edit_history.redo(&mut self.buffer) {
                    self.caret_state.restore(snapshot, &self.buffer);
                    true
                } else {
                    false
                }
            }
        }
    }

    fn selected_text(&self) -> Option<String> {
        let range = self.caret_state.selection_range()?;
        Some(self.buffer.text().slice(range).to_string())
    }

    fn insert_or_replace_selection(&mut self, text: &str) -> bool {
        let range = self
            .caret_state
            .selection_range()
            .unwrap_or(self.caret_state.caret_char()..self.caret_state.caret_char());
        self.apply_edit(range.start, range.end, text)
    }

    fn delete_selection(&mut self) -> bool {
        let Some(range) = self.caret_state.selection_range() else {
            return false;
        };
        self.apply_edit(range.start, range.end, "")
    }

    fn delete_backward(&mut self) -> bool {
        if self.caret_state.selection_range().is_some() {
            return self.delete_selection();
        }
        let caret = self.caret_state.caret_char();
        if caret == 0 {
            return false;
        }
        self.apply_edit(caret - 1, caret, "")
    }

    fn delete_forward(&mut self) -> bool {
        if self.caret_state.selection_range().is_some() {
            return self.delete_selection();
        }
        let caret = self.caret_state.caret_char();
        let total_chars = self.buffer.text().len_chars();
        if caret >= total_chars {
            return false;
        }
        self.apply_edit(caret, caret + 1, "")
    }

    fn apply_edit(&mut self, start: usize, end: usize, inserted_text: &str) -> bool {
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
        true
    }

    fn indentation_for_newline(buffer: &Buffer, caret_char: usize) -> String {
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
}
