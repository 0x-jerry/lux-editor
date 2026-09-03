//! Chrome domain: shell navigation, command panel, about window and the
//! theme-refresh flags.

use super::App;
use crate::app::input::EditorCommand;
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
    pub(crate) needs_style_refresh: bool,
    /// Whether the last applied chrome theme was dark; drives live `Auto` following.
    pub(crate) applied_theme_dark: Option<bool>,
    /// OS theme as reported last frame; feeds syntax-theme derivation in `Auto`.
    pub(crate) last_system_theme: Option<egui::Theme>,
}

impl App {
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