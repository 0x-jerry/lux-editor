//! Chrome domain: shell navigation, command panel, about window, and the style
//! (theme + fonts) the app pushes to egui.

use super::App;
use crate::app::input::EditorCommand;
use crate::ui::theme::{self, ThemeChoice};
use crate::ui::{AboutWindow, CommandPanel, Shell};
use eframe::egui;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellView {
    Editor,
    Configuration,
}

/// Actions exposed by the title-bar menus; the widgets render them, the app
/// maps them onto its own command/event pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TitleBarMenu {
    // File
    OpenFile,
    OpenFolder,
    SaveFile,
    // Edit
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    // View
    CommandPalette,
    SwitchToEditor,
    SwitchToConfiguration,
    ToggleSidebar,
    // Help
    About,
}

#[derive(Default)]
pub(crate) struct Chrome {
    pub(crate) shell: Shell,
    pub(crate) command_panel: CommandPanel,
    pub(crate) about_window: AboutWindow,
    /// Style (chrome visuals + fonts) must be re-pushed to egui on this `logic`
    /// pass; set by config reloads and by theme drift under `Auto`.
    pub(crate) needs_style_refresh: bool,
    /// Last theme actually applied to the egui context, with `Auto` already
    /// collapsed against the OS theme; `None` until the first frame.
    pub(crate) runtime_theme: Option<ThemeChoice>,
}

impl App {
    /// Push a resolved theme's chrome visuals + fonts to egui.
    pub(super) fn apply_style(&mut self, ctx: &egui::Context, resolved: ThemeChoice) {
        self.chrome.runtime_theme = Some(resolved);
        theme::apply_editor_settings(ctx, resolved, &self.settings.editor_config.settings);
    }

    pub(super) fn on_title_bar_menu(&mut self, menu: TitleBarMenu, ctx: &egui::Context) {
        match menu {
            TitleBarMenu::OpenFile => {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.open_file(path, ctx);
                }
            }
            TitleBarMenu::OpenFolder => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.open_folder(path);
                }
            }
            TitleBarMenu::SaveFile => {
                self.save_current_buffer(ctx);
            }
            TitleBarMenu::Undo => {
                self.execute_command(EditorCommand::Undo, ctx);
            }
            TitleBarMenu::Redo => {
                self.execute_command(EditorCommand::Redo, ctx);
            }
            TitleBarMenu::Cut => {
                self.execute_command(EditorCommand::Cut, ctx);
            }
            TitleBarMenu::Copy => {
                self.execute_command(EditorCommand::Copy, ctx);
            }
            TitleBarMenu::Paste => {
                if let Some(text) = clipboard_text() {
                    self.execute_command(EditorCommand::Paste(text), ctx);
                }
            }
            TitleBarMenu::SelectAll => {
                self.execute_command(EditorCommand::SelectAll, ctx);
            }
            TitleBarMenu::CommandPalette => self.chrome.command_panel.toggle(),
            TitleBarMenu::SwitchToEditor => self.chrome.shell.switch_to_editor(),
            TitleBarMenu::SwitchToConfiguration => self.chrome.shell.switch_to_configuration(),
            TitleBarMenu::ToggleSidebar => self.chrome.shell.toggle_sidebar(),
            TitleBarMenu::About => self.chrome.about_window.open(),
        }
    }
}

fn clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}
