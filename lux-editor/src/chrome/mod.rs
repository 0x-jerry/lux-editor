//! Chrome domain: the shell frame and its overlays — title/status bars,
//! sidebar, command palette, about and welcome views — plus the navigation
//! state (`ShellView`) and the native-menu command funnel (`TitleBarMenu`).

mod reducer;
mod state;
pub(crate) mod ui;

pub(crate) use state::{Chrome, ShellView, TitleBarMenu};
pub(crate) use ui::{AppView, AppViewInput};
