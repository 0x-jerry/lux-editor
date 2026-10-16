//! Application actions, their key bindings and the menu bar.
//!
//! Every user-facing command is an [`Action`](gpui_kit::Action) here: the key
//! bindings and the menus are two front ends that dispatch the same actions,
//! and the root view is the single place that handles them.

use gpui_kit::component::GlobalState;
use gpui_kit::component::input::{Copy, Cut, Paste, Redo, SelectAll, Undo};
use gpui_kit::{App, KeyBinding, Menu, MenuItem, OwnedMenu, SharedString, actions};

actions!(
    lux,
    [
        NewFile,
        OpenFile,
        OpenFolder,
        Save,
        SaveAs,
        FormatFile,
        CloseTab,
        ToggleSidebar,
        ToggleMarkdownPreview,
        OpenCommandPalette,
        OpenSettings,
        ClearRecentItems,
        Quit,
    ]
);

/// The platform's primary shortcut modifier ("cmd" on macOS, "ctrl" elsewhere).
fn primary(key: &str) -> SharedString {
    if cfg!(target_os = "macos") {
        format!("cmd-{key}").into()
    } else {
        format!("ctrl-{key}").into()
    }
}

fn shift_primary(key: &str) -> SharedString {
    if cfg!(target_os = "macos") {
        format!("cmd-shift-{key}").into()
    } else {
        format!("ctrl-shift-{key}").into()
    }
}

/// Register the global application shortcuts and publish the menu bar.
///
/// Called once from [`super::run`]. Bindings carry no key context so they stay
/// active while a text input is focused; the editor's own bindings are more
/// specific and win over these where they overlap.
pub fn install(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new(&primary("n"), NewFile, None),
        KeyBinding::new(&primary("o"), OpenFile, None),
        KeyBinding::new(&shift_primary("o"), OpenFolder, None),
        KeyBinding::new(&primary("s"), Save, None),
        KeyBinding::new(&shift_primary("s"), SaveAs, None),
        KeyBinding::new(&primary("w"), CloseTab, None),
        KeyBinding::new(&primary("b"), ToggleSidebar, None),
        KeyBinding::new(&shift_primary("p"), ToggleMarkdownPreview, None),
        KeyBinding::new(&primary("p"), OpenCommandPalette, None),
        KeyBinding::new(&primary(","), OpenSettings, None),
        KeyBinding::new(&primary("q"), Quit, None),
    ]);

    GlobalState::global_mut(cx).set_app_menus(menus());
}

/// The application menu bar, shared by the in-window bar and, on macOS, the
/// native menu built from the same definitions.
pub fn menus() -> Vec<OwnedMenu> {
    vec![
        file_menu().owned(),
        edit_menu().owned(),
        view_menu().owned(),
    ]
}

fn file_menu() -> Menu {
    Menu::new("File").items(vec![
        MenuItem::action("New File", NewFile),
        MenuItem::action("Open File…", OpenFile),
        MenuItem::action("Open Folder…", OpenFolder),
        MenuItem::separator(),
        MenuItem::action("Save", Save),
        MenuItem::action("Save As…", SaveAs),
        MenuItem::action("Format File", FormatFile),
        MenuItem::separator(),
        MenuItem::action("Close Tab", CloseTab),
        MenuItem::separator(),
        MenuItem::action("Quit", Quit),
    ])
}

fn edit_menu() -> Menu {
    Menu::new("Edit").items(vec![
        MenuItem::action("Undo", Undo),
        MenuItem::action("Redo", Redo),
        MenuItem::separator(),
        MenuItem::action("Cut", Cut),
        MenuItem::action("Copy", Copy),
        MenuItem::action("Paste", Paste),
        MenuItem::separator(),
        MenuItem::action("Select All", SelectAll),
    ])
}

fn view_menu() -> Menu {
    Menu::new("View").items(vec![
        MenuItem::action("Command Palette", OpenCommandPalette),
        MenuItem::separator(),
        MenuItem::action("Toggle Sidebar", ToggleSidebar),
        MenuItem::action("Toggle Markdown Preview", ToggleMarkdownPreview),
        MenuItem::separator(),
        MenuItem::action("Settings", OpenSettings),
    ])
}
