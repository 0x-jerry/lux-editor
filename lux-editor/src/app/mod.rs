//! Composition root: the [`App`] struct, the async runtime that feeds it and
//! the eframe frame adapter. Domain state and logic live in the feature
//! modules (`documents`, `workspace`, `settings`, `highlighting`, `chrome`);
//! this module only wires them together. Renders nothing itself — the UI
//! lives with each domain and talks to the app through `crate::events`.
//!
//! Behaviour is layered so [`App`] stays a plain state container:
//!
//! - domain structs (`Documents`, `Workspace`, `SettingsState`,
//!   `Highlighting`, `Chrome`) own their state and pure transitions;
//! - [`Ctx`] is a short-lived bundle of `&mut` borrows handed to the
//!   [`actions`] modules, which implement the cross-domain behaviour (each
//!   action is an `impl Ctx` method);
//! - this module only constructs the domains, runs the eframe loop and
//!   forwards events into a fresh [`Ctx`] each pass.

mod actions;
mod context;
mod events;
mod runtime;
pub(crate) mod startup;
mod state;
mod update;

pub(crate) use context::Ctx;
pub(crate) use runtime::Runtime;
pub use state::App;
