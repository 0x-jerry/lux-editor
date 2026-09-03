//! Native integration: the macOS system menubar and the cross-platform tray
//! icon. Both report through [`TitleBarMenu`] so the app maps them onto its
//! existing command pipeline (`App::on_title_bar_menu`).

mod icon;
#[cfg(target_os = "macos")]
mod menubar;
mod tray;

use crate::app::TitleBarMenu;

/// Native chrome owned by the app: the installed menubar/tray handles (kept
/// alive or the OS objects are deallocated) plus the window-visibility state
/// the tray toggle label is derived from.
pub(crate) struct NativeChrome {
    pub(crate) window_visible: bool,
    tray: Option<tray_icon::TrayIcon>,
    toggle_item: Option<muda::MenuItem>,
    tray_attempted: bool,
    #[cfg(target_os = "macos")]
    menubar: Option<muda::Menu>,
}

impl Default for NativeChrome {
    fn default() -> Self {
        Self {
            window_visible: true,
            tray: None,
            toggle_item: None,
            tray_attempted: false,
            #[cfg(target_os = "macos")]
            menubar: None,
        }
    }
}

impl NativeChrome {
    /// Install the system menubar (macOS) and tray icon exactly once. Runs on
    /// the main thread from `App::logic`, where `NSApplication` already exists.
    pub(crate) fn install(&mut self) {
        #[cfg(target_os = "macos")]
        if self.menubar.is_none() {
            self.menubar = menubar::build();
        }
        if !self.tray_attempted {
            self.tray_attempted = true;
            (self.tray, self.toggle_item) = tray::build();
        }
    }

    /// Reflect the current window visibility in the tray toggle label.
    pub(crate) fn update_tray_label(&self) {
        if let Some(item) = &self.toggle_item {
            item.set_text(if self.window_visible {
                "Hide Lux"
            } else {
                "Show Lux"
            });
        }
    }

    /// Drain native menu/tray events into app commands.
    pub(crate) fn drain(&mut self) -> Vec<TitleBarMenu> {
        let mut commands = Vec::new();
        while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
            if let Some(command) = command_for_id(event.id().as_ref()) {
                commands.push(command);
            }
        }
        // Windows: left-click on the tray toggles the window. macOS and Linux
        // open the tray menu on left-click instead.
        #[cfg(target_os = "windows")]
        while let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
            if let tray_icon::TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                button_state: tray_icon::MouseButtonState::Up,
                ..
            } = event
            {
                commands.push(TitleBarMenu::ToggleWindowVisibility);
            }
        }
        commands
    }
}

/// Menu-item id for a command; the single source both menu builders and the
/// event drain key off.
pub(crate) fn command_id(command: TitleBarMenu) -> &'static str {
    match command {
        TitleBarMenu::OpenFile => "open-file",
        TitleBarMenu::OpenFolder => "open-folder",
        TitleBarMenu::SaveFile => "save",
        TitleBarMenu::Undo => "undo",
        TitleBarMenu::Redo => "redo",
        TitleBarMenu::Cut => "cut",
        TitleBarMenu::Copy => "copy",
        TitleBarMenu::Paste => "paste",
        TitleBarMenu::SelectAll => "select-all",
        TitleBarMenu::CommandPalette => "command-palette",
        TitleBarMenu::SwitchToEditor => "switch-editor",
        TitleBarMenu::SwitchToConfiguration => "switch-configuration",
        TitleBarMenu::ToggleSidebar => "toggle-sidebar",
        TitleBarMenu::About => "about",
        TitleBarMenu::Hide => "hide",
        TitleBarMenu::Quit => "quit",
        TitleBarMenu::ToggleWindowVisibility => "tray.toggle",
    }
}

fn command_for_id(id: &str) -> Option<TitleBarMenu> {
    Some(match id {
        "open-file" => TitleBarMenu::OpenFile,
        "open-folder" => TitleBarMenu::OpenFolder,
        "save" => TitleBarMenu::SaveFile,
        "undo" => TitleBarMenu::Undo,
        "redo" => TitleBarMenu::Redo,
        "cut" => TitleBarMenu::Cut,
        "copy" => TitleBarMenu::Copy,
        "paste" => TitleBarMenu::Paste,
        "select-all" => TitleBarMenu::SelectAll,
        "command-palette" => TitleBarMenu::CommandPalette,
        "switch-editor" => TitleBarMenu::SwitchToEditor,
        "switch-configuration" => TitleBarMenu::SwitchToConfiguration,
        "toggle-sidebar" => TitleBarMenu::ToggleSidebar,
        "about" => TitleBarMenu::About,
        "hide" => TitleBarMenu::Hide,
        "quit" => TitleBarMenu::Quit,
        "tray.toggle" => TitleBarMenu::ToggleWindowVisibility,
        _ => return None,
    })
}