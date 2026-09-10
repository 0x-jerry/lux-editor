//! Editing actions: route raw egui input and editor messages toward the
//! active text tab, and perform the handful of global side effects command
//! execution needs (focus gating, clipboard, save, window title). The document
//! itself applies every `EditorCommand` through
//! [`OpenDocument::execute_command`](crate::document::OpenDocument); this
//! module only orchestrates it and decides when the editor (rather than a
//! panel or the configuration view) owns the keyboard.

use crate::app::Ctx;
use crate::document::{CommandOutcome, EditorCommand, commands_from_event};
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
                    active_document
                        .set_caret_from_pointer(line_index, column, selecting, add_cursor);
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

        let changed = match self
            .tabs
            .active_text_mut()
            .map_or(CommandOutcome::Noop, |doc| doc.execute_command(command))
        {
            CommandOutcome::Copy(text) => {
                egui.copy_text(text);
                false
            }
            CommandOutcome::Cut(text) => {
                egui.copy_text(text);
                true
            }
            CommandOutcome::Save => self.save_current_buffer(),
            CommandOutcome::Changed => true,
            CommandOutcome::Noop => false,
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
