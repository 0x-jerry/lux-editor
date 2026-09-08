//! Settings domain: the user configuration schema, its on-disk store, the
//! app-side state that watches the config file and the reactions to changes,
//! plus the configuration view (autosaving settings editor).

pub(crate) mod configuration_view;
pub(crate) mod schema;
mod state;
mod store;

pub(crate) use schema::{EditorSettings, WorkspaceSession};
pub(crate) use state::SettingsState;
pub(crate) use store::Config;
