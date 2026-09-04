//! Chrome reducer: shell navigation, palette toggling and title-bar menus.

use crate::app::App;
use crate::events::ShellEvent;
use eframe::egui;

impl App {
    /// Shell & navigation: view switching and title-bar actions.
    pub(crate) fn handle_shell_event(
        &mut self,
        event: ShellEvent,
        // macOS only consumes `ctx` in the platform-gated menu arm below.
        #[cfg_attr(target_os = "macos", allow(unused_variables))] ctx: &egui::Context,
    ) {
        match event {
            ShellEvent::SwitchToEditor => self.chrome.shell.switch_to_editor(),
            ShellEvent::SwitchToConfiguration => self.chrome.shell.switch_to_configuration(),
            ShellEvent::ToggleSidebar => self.chrome.shell.toggle_sidebar(),
            ShellEvent::ToggleCommandPanel => self.chrome.command_panel.toggle(),
            #[cfg(not(target_os = "macos"))]
            ShellEvent::TitleBarMenu(menu) => self.on_title_bar_menu(menu, ctx),
        }
    }

}
