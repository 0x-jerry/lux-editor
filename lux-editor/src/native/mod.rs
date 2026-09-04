//! Native integration: the macOS system menubar and the cross-platform tray
//! icon. Both report through [`TitleBarMenu`] so the app maps them onto its
//! existing command pipeline (`App::on_title_bar_menu`).

mod icon;
#[cfg(target_os = "macos")]
mod menubar;
mod tray;

use std::sync::{Arc, Mutex};

use crate::app::TitleBarMenu;

/// Native chrome owned by the app: the installed menubar/tray handles (kept
/// alive or the OS objects are deallocated) plus the window-visibility state
/// the tray toggle label is derived from.
pub(crate) struct NativeChrome {
    pub(crate) window_visible: bool,
    tray: Option<tray_icon::TrayIcon>,
    toggle_item: Option<muda::MenuItem>,
    tray_attempted: bool,
    /// Menu events delivered by the [`muda`] handler set in [`NativeChrome::install`].
    menu_events: Option<Arc<Mutex<Vec<TitleBarMenu>>>>,
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
            menu_events: None,
            #[cfg(target_os = "macos")]
            menubar: None,
        }
    }
}

impl NativeChrome {
    /// Install the system menubar (macOS) and tray icon exactly once. Runs on
    /// the main thread from `App::logic`, where `NSApplication` already exists.
    pub(crate) fn install(&mut self, ctx: &eframe::egui::Context) {
        #[cfg(target_os = "macos")]
        if self.menubar.is_none() {
            self.menubar = menubar::build();
        }
        if !self.tray_attempted {
            self.tray_attempted = true;
            (self.tray, self.toggle_item) = tray::build();
        }
        // A hidden window may stop repainting (eframe's invisible-window pump is
        // throttled and can stall), which would leave tray clicks undrained. So
        // menu events wake the egui loop from here instead of polling a channel.
        if self.menu_events.is_none() {
            let pending = Arc::new(Mutex::new(Vec::new()));
            self.menu_events = Some(Arc::clone(&pending));
            let ctx = ctx.clone();
            muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
                if let Some(command) = command_for_id(event.id().as_ref()) {
                    pending.lock().unwrap().push(command);
                    ctx.request_repaint();
                }
            }));
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
        // Events land in the muda channel until the handler in
        // `install` is registered (first frame), then in the queue it feeds.
        while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
            if let Some(command) = command_for_id(event.id().as_ref()) {
                commands.push(command);
            }
        }
        if let Some(pending) = &self.menu_events {
            commands.extend(pending.lock().unwrap().drain(..));
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

/// macOS: an orderOut'd window can only be restored from a tray click while the
/// app is inactive by activating the app first (`makeKeyAndOrderFront` is a
/// no-op otherwise, and winit's focus skips invisible windows). Must run on the
/// main thread; `App::logic` does.
#[cfg(target_os = "macos")]
pub(crate) fn activate_app() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationOptions, NSRunningApplication};
    if let Some(mtm) = MainThreadMarker::new() {
        if objc2::available!(macos = 14.0) {
            // Replaces the deprecated `activateIgnoringOtherApps:`.
            NSApplication::sharedApplication(mtm).activate();
        } else {
            // `activateWithOptions` itself is not deprecated; only the
            // `ActivateIgnoringOtherApps` constant is (a no-op on 14+), so
            // spell its bit out here where it still matters.
            NSRunningApplication::currentApplication()
                .activateWithOptions(NSApplicationActivationOptions(1 << 1));
        }
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
