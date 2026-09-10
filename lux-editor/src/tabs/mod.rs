//! Tab domain: the generic tab host. Owns the tab list and its life cycle
//! (`state`), the shared tab strip (`strip`) and the editor-area content
//! dispatch (`ui`). Text-specific machinery lives in [`crate::document`];
//! this module only manages tabs and renders whatever the active one holds.

mod state;
mod strip;
pub(crate) mod ui;

pub(crate) use state::{TabManager, TabMeta, openable_tab, tab_with_path};
pub(crate) use strip::{TabStripInput, TabStripView};
pub use ui::{EditorView, EditorViewState};
