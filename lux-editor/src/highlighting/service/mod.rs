mod client;
mod code;
mod engine;
mod languages;
mod parse;
mod snapshot;
mod style;
mod worker;

pub use client::HighlightingService;
pub(crate) use code::CodeHighlightEngine;
pub use languages::LanguageKind;
pub use snapshot::{HighlightSnapshot, HighlightSpan};
