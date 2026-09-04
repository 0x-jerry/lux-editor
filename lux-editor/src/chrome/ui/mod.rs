//! Chrome views built on the component pattern. They speak the app's event
//! language directly, so leaves stay thin and the reducers stay the only
//! place that mutates state.

pub(crate) mod widgets;

mod about;
mod app_view;
mod command_panel;
mod shell;
pub(crate) mod welcome;

pub use about::AboutWindow;
pub use app_view::{AppView, AppViewInput};
pub use command_panel::CommandPanel;
pub use shell::Shell;
