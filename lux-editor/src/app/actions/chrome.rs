//! Chrome actions: shell navigation, title-bar menus, tray/menubar commands
//! and the style pass (theme + fonts) that the frame drives.

use crate::app::Ctx;
use crate::chrome::TitleBarMenu;
use crate::document::EditorCommand;
use crate::events::ShellEvent;
use crate::theme::{self, CustomFont, StartupFont, ThemeChoice};
use eframe::egui;
use std::time::Duration;

impl Ctx<'_> {
    /// Shell & navigation: view switching and title-bar actions.
    pub(crate) fn handle_shell_event(&mut self, event: ShellEvent) {
        match event {
            ShellEvent::ToggleSidebar => self.chrome.shell.toggle_sidebar(),
            ShellEvent::ToggleCommandPanel => self.chrome.command_panel.toggle(),
            ShellEvent::ToggleMarkdownPreview => self.chrome.shell.toggle_markdown_preview(),
        }
    }

    /// Push a resolved theme's chrome visuals + fonts to egui.
    pub(crate) fn apply_style(&mut self, resolved: ThemeChoice) {
        self.chrome.runtime_theme = Some(resolved);
        // The preloaded bytes only apply to the family they were spawned for;
        // any other family falls back to the sync lookup. While the loader is
        // still in flight the chrome renders with system fallbacks and the
        // refresh stays armed, so the fold-in happens on a later pass.
        let settings = &self.settings.editor_config.settings;
        let mut font = CustomFont::Sync;
        let loader_matches = self
            .chrome
            .startup_font
            .as_ref()
            .is_some_and(|loader| loader.family == settings.font.family);
        if loader_matches {
            let polled = self
                .chrome
                .startup_font
                .as_mut()
                .and_then(StartupFont::poll);
            match polled {
                Some(bytes) => {
                    font = CustomFont::Preloaded(bytes);
                    self.chrome.startup_font = None;
                }
                None => {
                    font = CustomFont::Pending;
                    self.chrome.needs_style_refresh = true;
                    self.egui_ctx()
                        .request_repaint_after(Duration::from_millis(30));
                }
            }
        } else {
            self.chrome.startup_font = None;
        }
        theme::apply_editor_settings(self.egui_ctx(), resolved, settings, font);
        crate::app::startup::stage_once!("first style applied");
    }

    pub(crate) fn on_title_bar_menu(&mut self, menu: TitleBarMenu) {
        match menu {
            TitleBarMenu::OpenFile => {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.open_file(path);
                }
            }
            TitleBarMenu::OpenFolder => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.open_folder(path);
                }
            }
            TitleBarMenu::SaveFile => {
                self.save_current_buffer();
            }
            TitleBarMenu::Undo => self.edit_action(EditorCommand::Undo),
            TitleBarMenu::Redo => self.edit_action(EditorCommand::Redo),
            TitleBarMenu::Cut => self.edit_action(EditorCommand::Cut),
            TitleBarMenu::Copy => self.edit_action(EditorCommand::Copy),
            TitleBarMenu::Paste => {
                if let Some(text) = clipboard_text() {
                    self.edit_action(EditorCommand::Paste(text));
                }
            }
            TitleBarMenu::SelectAll => self.edit_action(EditorCommand::SelectAll),
            TitleBarMenu::CommandPalette => self.chrome.command_panel.toggle(),
            TitleBarMenu::ToggleSidebar => self.chrome.shell.toggle_sidebar(),
            TitleBarMenu::Hide => {
                self.chrome.native.window_visible = false;
                let egui = self.egui_ctx().clone();
                egui.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            TitleBarMenu::Quit => {
                self.egui_ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Close);
            }
            TitleBarMenu::ToggleWindowVisibility => {
                let visible = !self.chrome.native.window_visible;
                self.chrome.native.window_visible = visible;
                let egui = self.egui_ctx().clone();
                egui.send_viewport_cmd(egui::ViewportCommand::Visible(visible));
                if visible {
                    // macOS: an inactive app cannot restore an orderOut'd
                    // window; activate before the viewport commands run.
                    #[cfg(target_os = "macos")]
                    crate::native::activate_app();
                    egui.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
            }
            TitleBarMenu::About => self.chrome.about_window.open(),
        }
    }

    /// Send an edit action to whatever owns the keyboard. The code editor
    /// consumes [`EditorCommand`]s; a focused egui text input only reacts to
    /// input events, and the native menu swallowed the key before egui saw it,
    /// so the equivalent event is synthesized for the current pass.
    fn edit_action(&mut self, command: EditorCommand) {
        let egui = self.egui_ctx().clone();
        if egui.text_edit_focused()
            && let Some(event) = text_edit_event(&command)
        {
            egui.input_mut(|input| input.events.push(event));
            return;
        }
        self.execute_command(command);
    }

    /// One pass of the native menubar/tray: install once, reflect the tray
    /// label, then route any pending native commands through the same pipeline
    /// as the rendered chrome.
    pub(crate) fn native_menu_pass(&mut self) {
        let egui = self.egui_ctx().clone();
        self.chrome.native.install(&egui);
        self.chrome.native.update_tray_label();
        let native_commands = self.chrome.native.drain();
        for command in native_commands {
            self.on_title_bar_menu(command);
        }
    }
}

fn clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// The egui input event a focused [`egui::TextEdit`] handles for `command`.
fn text_edit_event(command: &EditorCommand) -> Option<egui::Event> {
    let key = |key: egui::Key, modifiers: egui::Modifiers| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    Some(match command {
        EditorCommand::SelectAll => key(egui::Key::A, egui::Modifiers::COMMAND),
        EditorCommand::Undo => key(egui::Key::Z, egui::Modifiers::COMMAND),
        EditorCommand::Redo => key(egui::Key::Z, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT),
        EditorCommand::Cut => egui::Event::Cut,
        EditorCommand::Copy => egui::Event::Copy,
        EditorCommand::Paste(text) => egui::Event::Paste(text.clone()),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn edit_commands_map_to_the_events_a_text_edit_handles() {
        assert_eq!(
            text_edit_event(&EditorCommand::SelectAll),
            Some(key(egui::Key::A, egui::Modifiers::COMMAND))
        );
        assert_eq!(
            text_edit_event(&EditorCommand::Undo),
            Some(key(egui::Key::Z, egui::Modifiers::COMMAND))
        );
        assert_eq!(
            text_edit_event(&EditorCommand::Redo),
            Some(key(
                egui::Key::Z,
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT
            ))
        );
        assert_eq!(text_edit_event(&EditorCommand::Cut), Some(egui::Event::Cut));
        assert_eq!(
            text_edit_event(&EditorCommand::Copy),
            Some(egui::Event::Copy)
        );
        assert_eq!(
            text_edit_event(&EditorCommand::Paste("x".to_string())),
            Some(egui::Event::Paste("x".to_string()))
        );
        assert_eq!(text_edit_event(&EditorCommand::Save), None);
    }
}

#[cfg(test)]
mod scratch {
    #[test]
    fn two_pastes_one_pass() {
        let ctx = eframe::egui::Context::default();
        let mut text = String::new();
        let id = eframe::egui::Id::new("t");
        let mut out = ctx.run_ui(eframe::egui::RawInput::default(), |ui| {
            ui.add(eframe::egui::TextEdit::singleline(&mut text).id(id))
                .request_focus();
        });
        out.textures_delta.clear();
        let mut raw = eframe::egui::RawInput::default();
        raw.events
            .push(eframe::egui::Event::Paste("foo".to_string()));
        raw.events
            .push(eframe::egui::Event::Paste("bar".to_string()));
        let mut out = ctx.run_ui(raw, |ui| {
            ui.add(eframe::egui::TextEdit::singleline(&mut text).id(id));
        });
        out.textures_delta.clear();
        assert_eq!(text, "foobar");
    }
}
