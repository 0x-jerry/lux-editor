//! UI layer: the `Component` pattern, the shell, app-specific views, reusable
//! widgets and theming. All egui rendering happens inside this module.

mod component;
mod components;
mod highlight;
pub mod theme;
mod types;
mod widgets;

pub use component::Component;
pub use components::{AboutWindow, AppView, AppViewInput, CommandPanel, Shell};
pub use types::DocumentTab;
