//! Filesystem locations the editor owns: config, recent state and logs.

use std::path::PathBuf;

/// Everything the editor writes itself.
pub(crate) fn data_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".lux-editor")
}
