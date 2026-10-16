//! Markdown preview: the rendered document, read-only, in a scrollable pane.

use gpui_kit::component::text::TextView;
use gpui_kit::*;
use std::path::Path;

/// Whether `path` is a markdown document, and so has a rendered view.
pub fn is_markdown(path: &Path) -> bool {
    const EXTENSIONS: [&str; 2] = ["md", "markdown"];
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .is_some_and(|extension| EXTENSIONS.contains(&extension.as_str()))
}

/// Render `text` as GitHub-flavoured markdown. `id` keeps the element identity
/// stable so scroll position and selection survive a re-render.
pub fn preview(id: u64, text: &str) -> AnyElement {
    div()
        .id(("markdown-preview", id as usize))
        .size_full()
        .overflow_y_scroll()
        .p_4()
        .child(TextView::markdown(("markdown", id as usize), text.to_string()))
        .into_any_element()
}
