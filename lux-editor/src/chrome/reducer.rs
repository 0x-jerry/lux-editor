//! Chrome reducer: shell navigation, palette toggling and title-bar menus.

use crate::app::App;
use crate::events::ShellEvent;

impl App {
    /// Shell & navigation: view switching and title-bar actions.
    pub(crate) fn handle_shell_event(&mut self, event: ShellEvent) {
        match event {
            ShellEvent::SwitchToEditor => self.chrome.shell.switch_to_editor(),
            ShellEvent::SwitchToConfiguration => self.chrome.shell.switch_to_configuration(),
            ShellEvent::ToggleSidebar => self.chrome.shell.toggle_sidebar(),
            ShellEvent::ToggleCommandPanel => self.chrome.command_panel.toggle(),
        }
    }
}
