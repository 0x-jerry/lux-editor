//! macOS system menubar: the app-rendered title-bar menu moved to the native
//! menu, in standard macOS layout (app menu first, ⌘-shortcuts on items).
//! macOS-only; other platforms keep the in-window menu bar.

use crate::app::TitleBarMenu;
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};

use super::command_id;

/// Builds the top-level menu and installs it as the application main menu.
pub(crate) fn build() -> Option<Menu> {
    let menu = Menu::new();
    let app_menu = app_menu()?;
    let file_menu = file_menu()?;
    let edit_menu = edit_menu()?;
    let view_menu = view_menu()?;

    menu.append(&app_menu).ok()?;
    menu.append(&file_menu).ok()?;
    menu.append(&edit_menu).ok()?;
    menu.append(&view_menu).ok()?;

    menu.init_for_nsapp();
    Some(menu)
}

/// Bold application menu: the first top-level menu on macOS.
fn app_menu() -> Option<Submenu> {
    let menu = Submenu::new("Lux", true);
    menu.append(&command_item(TitleBarMenu::About, "About Lux", None))
        .ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&command_item(TitleBarMenu::Hide, "Hide Lux", cmd(Code::KeyH)))
        .ok()?;
    // Hide Others / Show All target the app itself; the system handles them.
    menu.append(&PredefinedMenuItem::hide_others(Some("Hide Others")))
        .ok()?;
    menu.append(&PredefinedMenuItem::show_all(Some("Show All"))).ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&command_item(TitleBarMenu::Quit, "Quit Lux", cmd(Code::KeyQ)))
        .ok()?;
    Some(menu)
}

fn file_menu() -> Option<Submenu> {
    let menu = Submenu::new("File", true);
    menu.append(&command_item(TitleBarMenu::OpenFile, "Open File…", cmd(Code::KeyO)))
        .ok()?;
    menu.append(&command_item(
        TitleBarMenu::OpenFolder,
        "Open Folder…",
        cmd_shift(Code::KeyO),
    ))
    .ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&command_item(TitleBarMenu::SaveFile, "Save", cmd(Code::KeyS)))
        .ok()?;
    Some(menu)
}

fn edit_menu() -> Option<Submenu> {
    let menu = Submenu::new("Edit", true);
    menu.append(&command_item(TitleBarMenu::Undo, "Undo", cmd(Code::KeyZ))).ok()?;
    menu.append(&command_item(TitleBarMenu::Redo, "Redo", cmd_shift(Code::KeyZ)))
        .ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&command_item(TitleBarMenu::Cut, "Cut", cmd(Code::KeyX))).ok()?;
    menu.append(&command_item(TitleBarMenu::Copy, "Copy", cmd(Code::KeyC)))
        .ok()?;
    menu.append(&command_item(TitleBarMenu::Paste, "Paste", cmd(Code::KeyV)))
        .ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&command_item(TitleBarMenu::SelectAll, "Select All", cmd(Code::KeyA)))
        .ok()?;
    Some(menu)
}

fn view_menu() -> Option<Submenu> {
    let menu = Submenu::new("View", true);
    menu.append(&command_item(
        TitleBarMenu::CommandPalette,
        "Command Palette",
        cmd(Code::KeyK),
    ))
    .ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&command_item(TitleBarMenu::SwitchToEditor, "Editor", None))
        .ok()?;
    menu.append(&command_item(TitleBarMenu::SwitchToConfiguration, "Configuration", None))
        .ok()?;
    menu.append(&command_item(
        TitleBarMenu::ToggleSidebar,
        "Toggle Sidebar",
        cmd(Code::KeyB),
    ))
    .ok()?;
    Some(menu)
}

fn command_item(
    command: TitleBarMenu,
    text: &str,
    accelerator: Option<Accelerator>,
) -> MenuItem {
    MenuItem::with_id(command_id(command), text, true, accelerator)
}

fn cmd(code: Code) -> Option<Accelerator> {
    Some(Accelerator::new(Some(Modifiers::SUPER), code))
}

fn cmd_shift(code: Code) -> Option<Accelerator> {
    Some(Accelerator::new(Some(Modifiers::SUPER | Modifiers::SHIFT), code))
}