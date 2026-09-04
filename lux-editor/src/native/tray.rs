//! Cross-platform tray icon with a minimal menu (show/hide the window, quit).

use super::{command_id, icon};
use crate::app::TitleBarMenu;
use muda::{Menu, MenuItem, PredefinedMenuItem};

/// Builds the tray icon and its menu. Returns the icon handle (must be kept
/// alive or the OS object is removed) and the toggle item (for label updates).
pub(crate) fn build() -> (Option<tray_icon::TrayIcon>, Option<muda::MenuItem>) {
    let menu = Menu::new();
    let toggle = MenuItem::with_id(
        command_id(TitleBarMenu::ToggleWindowVisibility),
        "Show/Hide Lux",
        true,
        None,
    );
    let quit = MenuItem::with_id(command_id(TitleBarMenu::Quit), "Quit Lux", true, None);
    if menu.append(&toggle).is_ok()
        && menu.append(&PredefinedMenuItem::separator()).is_ok()
        && menu.append(&quit).is_ok()
    {
        let mut builder = tray_icon::TrayIconBuilder::new()
            .with_id("lux-tray")
            .with_tooltip("Lux Editor")
            .with_menu(Box::new(menu));
        if let Some(icon) = icon::tray_icon() {
            builder = builder.with_icon(icon).with_icon_as_template(true);
        }
        // Windows convention: left-click toggles the window, right-click opens
        // the menu. macOS/Linux always open the menu on click.
        #[cfg(target_os = "windows")]
        {
            builder = builder.with_menu_on_left_click(false);
        }
        return (builder.build().ok(), Some(toggle));
    }
    (None, None)
}
