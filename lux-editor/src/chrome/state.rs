//! Chrome domain: shell navigation, command panel, about window, and the style
//! (theme + fonts) the app pushes to egui.

use crate::chrome::ui::{AboutWindow, ClosePrompt, CommandPanel, Shell};
use crate::native::NativeChrome;
use crate::theme::StartupFont;
use crate::theme::ThemeChoice;

/// Actions exposed by the title-bar menus; the widgets render them, the app
/// maps them onto its own command/event pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TitleBarMenu {
    // File
    OpenFile,
    OpenFolder,
    CloseTab,
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
    ToggleSidebar,
    // Window (native menubar/tray only)
    Hide,
    Quit,
    ToggleWindowVisibility,
    // Help
    About,
}

#[derive(Default)]
pub(crate) struct Chrome {
    pub(crate) shell: Shell,
    pub(crate) command_panel: CommandPanel,
    pub(crate) about_window: AboutWindow,
    pub(crate) close_prompt: ClosePrompt,
    pub(crate) native: NativeChrome,
    /// Background font loader for the configured editor family; kept once it
    /// resolves so later style passes reuse the bytes instead of repeating the
    /// system lookup.
    pub(crate) startup_font: Option<StartupFont>,
    /// Style (chrome visuals + fonts) must be re-pushed to egui on this `logic`
    /// pass; set by config reloads and by theme drift under `Auto`.
    pub(crate) needs_style_refresh: bool,
    /// Last theme actually applied to the egui context, with `Auto` already
    /// collapsed against the OS theme; `None` until the first frame.
    pub(crate) runtime_theme: Option<ThemeChoice>,
}
