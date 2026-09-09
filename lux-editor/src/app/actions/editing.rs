//! Editing actions: turn raw egui input and editor messages into edits on the
//! active text tab, and decide when the editor (rather than a panel or the
//! configuration view) owns the keyboard.

use crate::app::Ctx;
use crate::document::{EditorCommand, commands_from_event};
use crate::events::EditingEvent;

impl Ctx<'_> {
    pub(crate) fn handle_editing_event(&mut self, event: EditingEvent) {
        match event {
            EditingEvent::SetCaretFromPointer {
                line_index,
                column,
                selecting,
                add_cursor,
            } => {
                if let Some(active_document) = self.tabs.active_text_mut() {
                    active_document.set_caret_from_pointer(
                        line_index,
                        column,
                        selecting,
                        add_cursor,
                    );
                }
            }
            EditingEvent::SelectWordFromPointer { line_index, column } => {
                if let Some(active_document) = self.tabs.active_text_mut() {
                    active_document.select_word_from_pointer(line_index, column);
                }
            }
        }
    }

    /// Route raw keyboard events into editor commands. Returns nothing; state
    /// changes happen through [`Ctx::execute_command`].
    pub(crate) fn handle_keyboard_input(&mut self) {
        let mut changed = false;
        let events = self.egui_ctx().input(|input| input.events.clone());
        for event in events {
            for command in commands_from_event(event) {
                if self.execute_command(command) {
                    changed = true;
                }
            }
        }

        if changed {
            self.schedule_language_refresh();
        }
    }

    pub(crate) fn execute_command(&mut self, command: EditorCommand) -> bool {
        if matches!(&command, EditorCommand::ToggleCommandPanel) {
            self.chrome.command_panel.toggle();
            return false;
        }

        if self.should_ignore_editor_command(&command) {
            return false;
        }

        if !matches!(&command, EditorCommand::Copy) {
            self.tabs.touch_caret_blink();
        }
        let egui = self.egui_ctx().clone();

        let changed = match command {
            EditorCommand::InsertText(text) => {
                self.tabs
                    .active_text_mut()
                    .is_some_and(|doc| doc.insert_or_replace_selection(&text))
            }
            EditorCommand::Paste(text) => {
                self.tabs
                    .active_text_mut()
                    .is_some_and(|doc| doc.insert_or_replace_selection(&text))
            }
            EditorCommand::InsertNewline => self
                .tabs
                .active_text_mut()
                .is_some_and(|doc| doc.insert_newline()),
            EditorCommand::InsertTab => self
                .tabs
                .active_text_mut()
                .is_some_and(|doc| doc.insert_or_replace_selection("    ")),
            EditorCommand::Backspace => self
                .tabs
                .active_text_mut()
                .is_some_and(|doc| doc.delete_backward()),
            EditorCommand::Delete => self
                .tabs
                .active_text_mut()
                .is_some_and(|doc| doc.delete_forward()),
            EditorCommand::DeleteWordBackward => self
                .tabs
                .active_text_mut()
                .is_some_and(|doc| doc.delete_word_backward()),
            EditorCommand::DeleteWordForward => self
                .tabs
                .active_text_mut()
                .is_some_and(|doc| doc.delete_word_forward()),
            EditorCommand::MoveLeft { selecting } => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                active_document
                    .caret_state
                    .move_left(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveRight { selecting } => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                active_document
                    .caret_state
                    .move_right(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveWordLeft { selecting } => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                active_document
                    .caret_state
                    .move_word_left(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveWordRight { selecting } => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                active_document
                    .caret_state
                    .move_word_right(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveUp { selecting } => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                active_document
                    .caret_state
                    .move_up(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveDown { selecting } => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                active_document
                    .caret_state
                    .move_down(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveHome { selecting } => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                active_document
                    .caret_state
                    .move_home(&active_document.buffer, selecting);
                false
            }
            EditorCommand::MoveEnd { selecting } => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                active_document
                    .caret_state
                    .move_end(&active_document.buffer, selecting);
                false
            }
            EditorCommand::SelectAll => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                active_document
                    .caret_state
                    .select_all(&active_document.buffer);
                false
            }
            EditorCommand::Copy => {
                if let Some(selected_text) = self
                    .tabs
                    .active_text()
                    .and_then(|doc| doc.selected_text())
                {
                    egui.copy_text(selected_text);
                }
                false
            }
            EditorCommand::Cut => {
                if let Some(removed) = self
                    .tabs
                    .active_text_mut()
                    .and_then(|doc| doc.cut_selection())
                {
                    egui.copy_text(removed);
                    true
                } else {
                    false
                }
            }
            EditorCommand::CollapseCarets => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                if active_document.caret_state.has_multiple_cursors() {
                    active_document.caret_state.remove_extra_cursors();
                    active_document.touch_caret_blink();
                }
                false
            }
            EditorCommand::AddCursorBelow => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                active_document
                    .caret_state
                    .add_cursor_below(&active_document.buffer);
                active_document.touch_caret_blink();
                false
            }
            EditorCommand::AddCursorAbove => {
                let Some(active_document) = self.tabs.active_text_mut() else {
                    return false;
                };
                active_document
                    .caret_state
                    .add_cursor_above(&active_document.buffer);
                active_document.touch_caret_blink();
                false
            }
            EditorCommand::Undo => {
                let snapshot = {
                    let Some(active_document) = self.tabs.active_text_mut() else {
                        return false;
                    };
                    active_document
                        .edit_history
                        .undo(&mut active_document.buffer)
                };
                if let Some(snapshot) = snapshot {
                    let Some(active_document) = self.tabs.active_text_mut() else {
                        return false;
                    };
                    active_document
                        .caret_state
                        .restore(snapshot, &active_document.buffer);
                    active_document.mark_dirty();
                    true
                } else {
                    false
                }
            }
            EditorCommand::Redo => {
                let snapshot = {
                    let Some(active_document) = self.tabs.active_text_mut() else {
                        return false;
                    };
                    active_document
                        .edit_history
                        .redo(&mut active_document.buffer)
                };
                if let Some(snapshot) = snapshot {
                    let Some(active_document) = self.tabs.active_text_mut() else {
                        return false;
                    };
                    active_document
                        .caret_state
                        .restore(snapshot, &active_document.buffer);
                    active_document.mark_dirty();
                    true
                } else {
                    false
                }
            }
            EditorCommand::Save => self.save_current_buffer(),
            EditorCommand::ToggleCommandPanel => false,
        };

        if changed {
            self.update_window_title();
        }
        changed
    }

    fn should_ignore_editor_command(&mut self, command: &EditorCommand) -> bool {
        if matches!(command, EditorCommand::ToggleCommandPanel) {
            return false;
        }

        let egui = self.egui_ctx().clone();
        let active_text = self.tabs.active_text();
        self.chrome.command_panel.open()
            || self.tabs.active_is_configuration()
            || active_text.is_none_or(|document| {
                document.missing || document.binary || !document.edit_area_focused
            })
            || egui.egui_wants_keyboard_input()
    }

    /// Whether the edit area has focus: the window is focused, we are on the
    /// editor view, and nothing else (command palette, file-tree rename, config
    /// field) is stealing keyboard input. If the window isn't focused, the edit
    /// area isn't either.
    pub(crate) fn editor_focused(&mut self) -> bool {
        let egui = self.egui_ctx().clone();
        self.tabs.active_text().is_some_and(|document| {
            document.edit_area_focused && !document.missing && !document.binary
        }) && egui.input(|i| i.focused)
            && !self.chrome.command_panel.open()
            && !self.tabs.active_is_configuration()
            && !egui.egui_wants_keyboard_input()
    }
}