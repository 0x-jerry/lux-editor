//! Reusable, app-agnostic egui widgets shared across the Lux shell.
//!
//! Components here must not depend on application types (buffers,
//! configuration, file trees, custom events). They communicate through small
//! message/enum returns that the app maps into its own domain. Moved into the
//! editor crate from the former `lux-ui` workspace crate.

pub mod shell;
pub mod title_bar;