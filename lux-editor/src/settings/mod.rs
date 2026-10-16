//! Settings domain: the user configuration schema and its on-disk store.

mod store;
pub(crate) mod types;

pub(crate) use store::Config;
pub(crate) use types::EditorSettings;
