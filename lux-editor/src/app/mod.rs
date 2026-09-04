//! Composition root: the `App` struct, the async runtime that feeds it and
//! the eframe frame adapter. Domain state and logic live in the feature
//! modules (`documents`, `workspace`, `settings`, `highlighting`, `chrome`);
//! this module only wires them together and dispatches events. Renders
//! nothing itself — the UI lives with each domain and talks to the app
//! through `crate::events::CustomEvent`.

mod events;
mod state;
pub(crate) mod startup;
mod update;

pub use state::App;
