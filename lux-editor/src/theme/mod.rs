//! Theme file format and the embedded built-in themes.
//!
//! A theme JSON file (see `assets/themes/`) answers two questions at once:
//! how the chrome looks and which colors highlight code, so token/background
//! contrast is guaranteed by construction. Built-in themes are embedded at
//! compile time with `include_str!`, like the highlight queries.

mod builtin;
mod choice;
mod color;
mod file;
mod visuals;

mod apply;
pub use apply::{CustomFont, StartupFont, apply_editor_settings};
pub use builtin::syntax_colors;
pub use choice::{resolve, ThemeChoice};
pub use file::SyntaxColors;
pub use visuals::AppTheme;
