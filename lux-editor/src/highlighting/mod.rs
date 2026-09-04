//! Highlighting domain: language parsing via tree-sitter in a background
//! service (`service`), the app-side debounced refresh (`state`) and the egui
//! paint helpers that turn snapshots into styled text (`paint`).

mod paint;
mod service;
mod state;

pub(crate) use paint::{build_highlighted_line_job, snapshot_color};
pub(crate) use service::{HighlightSnapshot, HighlightSpan, HighlightingService, LanguageKind};
pub(crate) use state::Highlighting;
