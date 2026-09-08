//! Chrome actions: shell navigation, title-bar menus, tray/menubar commands
//! and the style pass (theme + fonts) that the frame drives.

use crate::app::Ctx;
use crate::chrome::TitleBarMenu;
use crate::documents::EditorCommand;
use crate::events::ShellEvent;
use crate::theme::{self, CustomFont, StartupFont, ThemeChoice};
use eframe::egui;
use std::time::Duration;

impl Ctx<'_> {
    /// Shell & navigation: view switching and title-bar actions.
    pub(crate) fn handle_shell_event(&mut self, event: ShellEvent) {
        match event {
            ShellEvent::SwitchToEditor => self.chrome.shell.switch_to_editor(),
            ShellEvent::SwitchToConfiguration => self.chrome.shell.switch_to_configuration(),
            ShellEvent::ToggleSidebar => self.chrome.shell.toggle_sidebar(),
            ShellEvent::ToggleCommandPanel => self.chrome.command_panel.toggle(),
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
            TitleBarMenu::Undo => {
                self.execute_command(EditorCommand::Undo);
            }
            TitleBarMenu::Redo => {
                self.execute_command(EditorCommand::Redo);
            }
            TitleBarMenu::Cut => {
                self.execute_command(EditorCommand::Cut);
            }
            TitleBarMenu::Copy => {
                self.execute_command(EditorCommand::Copy);
            }
            TitleBarMenu::Paste => {
                if let Some(text) = clipboard_text() {
                    self.execute_command(EditorCommand::Paste(text));
                }
            }
            TitleBarMenu::SelectAll => {
                self.execute_command(EditorCommand::SelectAll);
            }
            TitleBarMenu::CommandPalette => self.chrome.command_panel.toggle(),
            TitleBarMenu::SwitchToEditor => self.chrome.shell.switch_to_editor(),
            TitleBarMenu::SwitchToConfiguration => self.chrome.shell.switch_to_configuration(),
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
