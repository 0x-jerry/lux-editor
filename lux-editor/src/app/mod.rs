//! Application logic: `App` state split into domain structs (`documents`,
//! `workspace`, `settings`, `highlighting`, `chrome`) plus the eframe frame
//! adapter. Renders nothing itself — the UI lives in `ui/` and talks to the
//! app through `crate::events::CustomEvent`.

mod chrome;
mod document;
mod documents;
mod events;
mod formatter;
mod highlighting;
mod input;
mod settings;
mod state;
mod update;
mod workspace;

pub use chrome::{ShellView, TitleBarMenu};
pub use document::OpenDocument;
pub use state::App;
