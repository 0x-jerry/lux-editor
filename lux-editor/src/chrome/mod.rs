//! Chrome domain: the shell frame and its overlays — title/status bars,
//! sidebar, command palette, about and welcome views — plus the native-menu
//! command funnel (`TitleBarMenu`).

mod state;
pub(crate) mod ui;

pub(crate) use state::{Chrome, TitleBarMenu};
pub(crate) use ui::{AppView, AppViewInput};
