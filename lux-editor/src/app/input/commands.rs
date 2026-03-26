use crate::app::App;
use eframe::egui;

pub(super) enum EditorCommand {
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
}

impl App {
    pub(in crate::app) fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
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
        if matches!(&command, EditorCommand::ToggleCommandPanel) {
            self.toggle_command_panel();
            return false;
        }

        if self.should_ignore_editor_command(&command, ctx) {
            return false;
        }

        if !matches!(&command, EditorCommand::Copy) {
            self.touch_caret_blink();
        }

        match command {
            EditorCommand::InsertText(text) | EditorCommand::Paste(text) => {
                self.insert_or_replace_selection(&text, ctx)
            }
            EditorCommand::InsertNewline => {
                let indentation =
                    Self::indentation_for_newline(&self.buffer, self.caret_state.caret_char());
                self.insert_or_replace_selection(&indentation, ctx)
            }
            EditorCommand::InsertTab => self.insert_or_replace_selection("    ", ctx),
            EditorCommand::Backspace => self.delete_backward(ctx),
            EditorCommand::Delete => self.delete_forward(ctx),
            EditorCommand::DeleteWordBackward => self.delete_word_backward(ctx),
            EditorCommand::DeleteWordForward => self.delete_word_forward(ctx),
            EditorCommand::MoveLeft { selecting } => {
                self.caret_state.move_left(&self.buffer, selecting);
                false
            }
            EditorCommand::MoveRight { selecting } => {
                self.caret_state.move_right(&self.buffer, selecting);
                false
            }
            EditorCommand::MoveWordLeft { selecting } => {
                self.caret_state.move_word_left(&self.buffer, selecting);
                false
            }
            EditorCommand::MoveWordRight { selecting } => {
                self.caret_state.move_word_right(&self.buffer, selecting);
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
                self.copy_selection_to_clipboard(ctx);
                false
            }
            EditorCommand::Cut => self.cut_selection_to_clipboard(ctx),
            EditorCommand::Undo => {
                if let Some(snapshot) = self.edit_history.undo(&mut self.buffer) {
                    self.caret_state.restore(snapshot, &self.buffer);
                    self.mark_document_dirty(ctx);
                    true
                } else {
                    false
                }
            }
            EditorCommand::Redo => {
                if let Some(snapshot) = self.edit_history.redo(&mut self.buffer) {
                    self.caret_state.restore(snapshot, &self.buffer);
                    self.mark_document_dirty(ctx);
                    true
                } else {
                    false
                }
            }
            EditorCommand::Save => self.save_current_buffer(ctx),
            EditorCommand::ToggleCommandPanel => false,
        }
    }
}
