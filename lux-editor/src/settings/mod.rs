//! Settings domain: the user configuration schema, its on-disk store and the
//! watcher that reloads it when the file changes.

mod store;
pub(crate) mod types;
mod watch;

pub(crate) use store::Config;
pub(crate) use types::EditorSettings;
pub(crate) use watch::watch;
