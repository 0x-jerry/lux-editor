use crate::document::OpenDocument;
use eframe::egui;

pub(crate) enum EditorCommand {
    InsertText(String),
    Paste(String),
    InsertNewline,
    InsertTab,
    Backspace,
    Delete,
    DeleteWordBackward,
    DeleteWordForward,
    MoveLeft { selecting: bool },
    MoveRight { selecting: bool },
    MoveWordLeft { selecting: bool },
    MoveWordRight { selecting: bool },
    MoveUp { selecting: bool },
    MoveDown { selecting: bool },
    MoveHome { selecting: bool },
    MoveEnd { selecting: bool },
    SelectAll,
    Copy,
    Cut,
    Undo,
    Redo,
    Save,
    ToggleCommandPanel,
    CollapseCarets,
    AddCursorBelow,
    AddCursorAbove,
}

pub(crate) fn commands_from_event(event: egui::Event) -> Vec<EditorCommand> {
    match event {
        egui::Event::Text(text) => {
            if text.starts_with(|c: char| c.is_ascii_control()) {
                Vec::new()
            } else {
                vec![EditorCommand::InsertText(text)]
            }
        }
        egui::Event::Paste(text) => vec![EditorCommand::Paste(text)],
        egui::Event::Copy => vec![EditorCommand::Copy],
        egui::Event::Cut => vec![EditorCommand::Cut],
        egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            let shortcut = modifiers.command || modifiers.ctrl;
            let selecting = modifiers.shift;
            if shortcut && modifiers.alt {
                return match key {
                    egui::Key::ArrowDown => vec![EditorCommand::AddCursorBelow],
                    egui::Key::ArrowUp => vec![EditorCommand::AddCursorAbove],
                    _ => Vec::new(),
                };
            }
            if shortcut && !modifiers.alt {
                return match key {
                    egui::Key::A => vec![EditorCommand::SelectAll],
                    egui::Key::C => vec![EditorCommand::Copy],
                    egui::Key::K => vec![EditorCommand::ToggleCommandPanel],
                    egui::Key::X => vec![EditorCommand::Cut],
                    egui::Key::S => vec![EditorCommand::Save],
                    egui::Key::Z if modifiers.shift => vec![EditorCommand::Redo],
                    egui::Key::Z => vec![EditorCommand::Undo],
                    egui::Key::Y => vec![EditorCommand::Redo],
                    _ => Vec::new(),
                };
            }
            if modifiers.alt {
                return match key {
                    egui::Key::ArrowLeft => vec![EditorCommand::MoveWordLeft { selecting }],
                    egui::Key::ArrowRight => vec![EditorCommand::MoveWordRight { selecting }],
                    egui::Key::Backspace => vec![EditorCommand::DeleteWordBackward],
                    egui::Key::Delete => vec![EditorCommand::DeleteWordForward],
                    _ => Vec::new(),
                };
            }
            match key {
                egui::Key::Enter => vec![EditorCommand::InsertNewline],
                egui::Key::Tab => vec![EditorCommand::InsertTab],
                egui::Key::Backspace => vec![EditorCommand::Backspace],
                egui::Key::Delete => vec![EditorCommand::Delete],
                egui::Key::Escape => vec![EditorCommand::CollapseCarets],
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

/// What the app must do *after* a document has handled an [`EditorCommand`].
/// The document applies every state change itself; this only reports the
/// side effects that need global plumbing (clipboard, save, chrome).
#[derive(Debug, PartialEq)]
pub(crate) enum CommandOutcome {
    /// Nothing observable happened (caret-only move, empty undo history, no
    /// selection to copy).
    Noop,
    /// The buffer changed; the caller should refresh chrome and highlighting.
    Changed,
    /// Copy: text to place on the clipboard; the buffer did not change.
    Copy(String),
    /// Cut: clipboard text, and the buffer did change.
    Cut(String),
    /// The user asked to save; the caller decides whether a save is possible
    /// (missing/binary paths, save-as dialog) and writes the file.
    Save,
}

impl OpenDocument {
    /// Run one editor command against this document: apply the edit or caret
    /// move directly, and report the side effects the app must perform. The
    /// app intercepts chrome-only commands (`ToggleCommandPanel`) and gates
    /// input (focus, panel, configuration tab) before this runs.
    pub(crate) fn execute_command(&mut self, command: EditorCommand) -> CommandOutcome {
        match command {
            EditorCommand::InsertText(text) | EditorCommand::Paste(text) => {
                changed(self.insert_or_replace_selection(&text))
            }
            EditorCommand::InsertNewline => changed(self.insert_newline()),
            EditorCommand::InsertTab => changed(self.insert_or_replace_selection("    ")),
            EditorCommand::Backspace => changed(self.delete_backward()),
            EditorCommand::Delete => changed(self.delete_forward()),
            EditorCommand::DeleteWordBackward => changed(self.delete_word_backward()),
            EditorCommand::DeleteWordForward => changed(self.delete_word_forward()),
            EditorCommand::MoveLeft { selecting } => {
                self.caret_state.move_left(&self.buffer, selecting);
                CommandOutcome::Noop
            }
            EditorCommand::MoveRight { selecting } => {
                self.caret_state.move_right(&self.buffer, selecting);
                CommandOutcome::Noop
            }
            EditorCommand::MoveWordLeft { selecting } => {
                self.caret_state.move_word_left(&self.buffer, selecting);
                CommandOutcome::Noop
            }
            EditorCommand::MoveWordRight { selecting } => {
                self.caret_state.move_word_right(&self.buffer, selecting);
                CommandOutcome::Noop
            }
            EditorCommand::MoveUp { selecting } => {
                self.caret_state.move_up(&self.buffer, selecting);
                CommandOutcome::Noop
            }
            EditorCommand::MoveDown { selecting } => {
                self.caret_state.move_down(&self.buffer, selecting);
                CommandOutcome::Noop
            }
            EditorCommand::MoveHome { selecting } => {
                self.caret_state.move_home(&self.buffer, selecting);
                CommandOutcome::Noop
            }
            EditorCommand::MoveEnd { selecting } => {
                self.caret_state.move_end(&self.buffer, selecting);
                CommandOutcome::Noop
            }
            EditorCommand::SelectAll => {
                self.caret_state.select_all(&self.buffer);
                CommandOutcome::Noop
            }
            EditorCommand::Copy => match self.selected_text() {
                Some(text) => CommandOutcome::Copy(text),
                None => CommandOutcome::Noop,
            },
            EditorCommand::Cut => match self.cut_selection() {
                Some(text) => CommandOutcome::Cut(text),
                None => CommandOutcome::Noop,
            },
            EditorCommand::CollapseCarets => {
                if self.caret_state.has_multiple_cursors() {
                    self.caret_state.remove_extra_cursors();
                    self.touch_caret_blink();
                }
                CommandOutcome::Noop
            }
            EditorCommand::AddCursorBelow => {
                self.caret_state.add_cursor_below(&self.buffer);
                self.touch_caret_blink();
                CommandOutcome::Noop
            }
            EditorCommand::AddCursorAbove => {
                self.caret_state.add_cursor_above(&self.buffer);
                self.touch_caret_blink();
                CommandOutcome::Noop
            }
            EditorCommand::Undo => {
                if let Some(snapshot) = self.edit_history.undo(&mut self.buffer) {
                    self.caret_state.restore(snapshot, &self.buffer);
                    self.mark_dirty();
                    CommandOutcome::Changed
                } else {
                    CommandOutcome::Noop
                }
            }
            EditorCommand::Redo => {
                if let Some(snapshot) = self.edit_history.redo(&mut self.buffer) {
                    self.caret_state.restore(snapshot, &self.buffer);
                    self.mark_dirty();
                    CommandOutcome::Changed
                } else {
                    CommandOutcome::Noop
                }
            }
            EditorCommand::Save => CommandOutcome::Save,
            // Intercepted by the app before dispatch; never reaches a document.
            EditorCommand::ToggleCommandPanel => CommandOutcome::Noop,
        }
    }
}

fn changed(edited: bool) -> CommandOutcome {
    if edited {
        CommandOutcome::Changed
    } else {
        CommandOutcome::Noop
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentBuffer, OpenDocument};

    fn document(text: &str) -> OpenDocument {
        let mut buffer = DocumentBuffer::new();
        buffer.set_path("/ws/a.rs");
        buffer.insert(0, text);
        OpenDocument::from_buffer(buffer)
    }

    #[test]
    fn insert_reports_changed_and_lands_in_buffer() {
        let mut doc = document("");
        assert_eq!(
            doc.execute_command(EditorCommand::InsertText("hi".to_string())),
            CommandOutcome::Changed
        );
        assert_eq!(doc.buffer.text().to_string(), "hi");
    }

    #[test]
    fn caret_move_is_noop_but_moves_the_caret() {
        let mut doc = document("hello");
        assert_eq!(
            doc.execute_command(EditorCommand::MoveRight { selecting: false }),
            CommandOutcome::Noop
        );
        assert_eq!(doc.caret_state.caret_char_at(0), 1);
        assert_eq!(
            doc.execute_command(EditorCommand::MoveEnd { selecting: true }),
            CommandOutcome::Noop
        );
        assert_eq!(doc.caret_state.caret_char_at(0), 5);
    }

    #[test]
    fn copy_returns_selection_or_noop() {
        let mut doc = document("ab");
        assert_eq!(
            doc.execute_command(EditorCommand::Copy),
            CommandOutcome::Noop
        );
        doc.execute_command(EditorCommand::SelectAll);
        assert_eq!(
            doc.execute_command(EditorCommand::Copy),
            CommandOutcome::Copy("ab".to_string())
        );
        // Copy never changes the buffer.
        assert_eq!(doc.buffer.text().to_string(), "ab");
    }

    #[test]
    fn cut_removes_selection_and_returns_text() {
        let mut doc = document("ab");
        doc.execute_command(EditorCommand::SelectAll);
        assert_eq!(
            doc.execute_command(EditorCommand::Cut),
            CommandOutcome::Cut("ab".to_string())
        );
        assert_eq!(doc.buffer.text().to_string(), "");
    }

    #[test]
    fn undo_redo_cycles_and_exhausts() {
        let mut doc = document("");
        doc.execute_command(EditorCommand::InsertText("hi".to_string()));
        assert_eq!(doc.buffer.text().to_string(), "hi");
        assert_eq!(
            doc.execute_command(EditorCommand::Undo),
            CommandOutcome::Changed
        );
        assert_eq!(doc.buffer.text().to_string(), "");
        assert_eq!(
            doc.execute_command(EditorCommand::Undo),
            CommandOutcome::Noop
        );
        assert_eq!(
            doc.execute_command(EditorCommand::Redo),
            CommandOutcome::Changed
        );
        assert_eq!(doc.buffer.text().to_string(), "hi");
        assert_eq!(
            doc.execute_command(EditorCommand::Redo),
            CommandOutcome::Noop
        );
    }

    #[test]
    fn save_is_reported_to_the_caller() {
        let mut doc = document("x");
        assert_eq!(
            doc.execute_command(EditorCommand::Save),
            CommandOutcome::Save
        );
    }

    #[test]
    fn multi_cursor_commands_stay_noop() {
        let mut doc = document("a\nb");
        assert_eq!(
            doc.execute_command(EditorCommand::AddCursorBelow),
            CommandOutcome::Noop
        );
        assert!(doc.caret_state.has_multiple_cursors());
        assert_eq!(
            doc.execute_command(EditorCommand::CollapseCarets),
            CommandOutcome::Noop
        );
        assert!(!doc.caret_state.has_multiple_cursors());
    }
}
