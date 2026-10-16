//! Theme file format and the embedded built-in themes.
//!
//! A theme JSON file (see `assets/themes/`) carries the chrome colors; the
//! built-in themes are embedded at compile time with `include_str!`.

mod builtin;
mod choice;
pub(crate) mod color;
mod file;

pub use choice::{ThemeChoice, resolve};
pub(crate) use builtin::builtin;
pub(crate) use file::ChromeColors;
