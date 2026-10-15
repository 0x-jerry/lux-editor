//! File extension to gpui-kit editor language name.

use std::path::Path;

/// The gpui-kit language name for `path`, or `None` for plain text.
pub fn language_for(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match extension.as_str() {
        "rs" => "rust",
        "js" | "mjs" | "cjs" | "jsx" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "md" | "markdown" => "markdown",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "sh" | "bash" => "bash",
        "html" | "htm" => "html",
        "py" | "pyi" => "python",
        "json" => "json",
        _ => return None,
    })
}
