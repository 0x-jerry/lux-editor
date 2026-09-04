use crate::app::App;
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

impl App {
    pub(crate) fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
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
            self.schedule_language_refresh();
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

    pub(crate) fn execute_command(&mut self, command: EditorCommand, ctx: &egui::Context) -> bool {
        if matches!(&command, EditorCommand::ToggleCommandPanel) {
            self.chrome.command_panel.toggle();
            return false;
        }

        if self.should_ignore_editor_command(&command, ctx) {
            return false;
        }

        if !matches!(&command, EditorCommand::Copy) {
            self.documents.touch_caret_blink();
        }

        match command {
            EditorCommand::InsertText(text) => self.insert_text_with_pairing(&text, ctx),
            EditorCommand::Paste(text) => self.insert_or_replace_selection(&text, ctx),
            EditorCommand::InsertNewline => self.insert_newline(ctx),
            EditorCommand::InsertTab => self.insert_or_replace_selection("    ", ctx),
            EditorCommand::Backspace => self.delete_backward(ctx),
            EditorCommand::Delete => self.delete_forward(ctx),
            EditorCommand::DeleteWordBackward => self.delete_word_backward(ctx),
            EditorCommand::DeleteWordForward => self.delete_word_forward(ctx),
            EditorCommand::MoveLeft { selecting } => {
                let active_document = self.active_document_mut();
                active_document
                    .caret_state
                    .move_left(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveRight { selecting } => {
                let active_document = self.active_document_mut();
                active_document
                    .caret_state
                    .move_right(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveWordLeft { selecting } => {
                let active_document = self.active_document_mut();
                active_document
                    .caret_state
                    .move_word_left(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveWordRight { selecting } => {
                let active_document = self.active_document_mut();
                active_document
                    .caret_state
                    .move_word_right(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveUp { selecting } => {
                let active_document = self.active_document_mut();
                active_document
                    .caret_state
                    .move_up(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveDown { selecting } => {
                let active_document = self.active_document_mut();
                active_document
                    .caret_state
                    .move_down(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveHome { selecting } => {
                let active_document = self.active_document_mut();
                active_document
                    .caret_state
                    .move_home(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveEnd { selecting } => {
                let active_document = self.active_document_mut();
                active_document
                    .caret_state
                    .move_end(&active_document.buffer, selecting);
                false
            }
            EditorCommand::SelectAll => {
                let active_document = self.active_document_mut();
                active_document
                    .caret_state
                    .select_all(&active_document.buffer);
                false
            }
            EditorCommand::Copy => {
                self.copy_selection_to_clipboard(ctx);
                false
            }
            EditorCommand::Cut => self.cut_selection_to_clipboard(ctx),
            EditorCommand::CollapseCarets => {
                let active_document = self.active_document_mut();
                if active_document.caret_state.has_multiple_cursors() {
                    active_document.caret_state.remove_extra_cursors();
                    self.documents.touch_caret_blink();
                }
                false
            }
            EditorCommand::AddCursorBelow => {
                let active_document = self.active_document_mut();
                active_document
                    .caret_state
                    .add_cursor_below(&active_document.buffer);
                self.documents.touch_caret_blink();
                false
            }
            EditorCommand::AddCursorAbove => {
                let active_document = self.active_document_mut();
                active_document
                    .caret_state
                    .add_cursor_above(&active_document.buffer);
                self.documents.touch_caret_blink();
                false
            }
            EditorCommand::Undo => {
                let snapshot = {
                    let active_document = self.active_document_mut();
                    active_document
                        .edit_history
                        .undo(&mut active_document.buffer)
                };
                if let Some(snapshot) = snapshot {
                    let active_document = self.active_document_mut();
                    active_document
                        .caret_state
                        .restore(snapshot, &active_document.buffer);
                    self.mark_document_dirty(ctx);
                    true
                } else {
                    false
                }
            }
            EditorCommand::Redo => {
                let snapshot = {
                    let active_document = self.active_document_mut();
                    active_document
                        .edit_history
                        .redo(&mut active_document.buffer)
                };
                if let Some(snapshot) = snapshot {
                    let active_document = self.active_document_mut();
                    active_document
                        .caret_state
                        .restore(snapshot, &active_document.buffer);
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
