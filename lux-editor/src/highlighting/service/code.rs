use ropey::Rope;

use super::LanguageKind;
use super::engine::Engines;
use super::parse::parse_snapshot;
use super::snapshot::HighlightSpan;
use crate::theme::SyntaxColors;

/// Synchronous tree-sitter highlighter for small snippets (markdown preview
/// fences). The document highlighter's worker owns its own `Engines` off the
/// UI thread, so this keeps a second set for on-frame callers.
pub(crate) struct CodeHighlightEngine {
    engines: Engines,
}

impl Default for CodeHighlightEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeHighlightEngine {
    pub(crate) fn new() -> Self {
        Self {
            engines: Engines::new(),
        }
    }

    pub(crate) fn line_tokens(
        &mut self,
        code: &str,
        language: LanguageKind,
        syntax: &SyntaxColors,
    ) -> Vec<Vec<HighlightSpan>> {
        parse_snapshot(
            &mut self.engines,
            syntax,
            &Rope::from_str(code),
            language,
            0,
        )
        .line_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{self, ThemeChoice};

    fn color_at(tokens: &[Vec<HighlightSpan>], line: usize, col: usize) -> Option<[u8; 4]> {
        tokens[line]
            .iter()
            .find(|span| span.start_col <= col && col < span.end_col)
            .map(|span| span.color)
    }

    #[test]
    fn rust_fence_matches_editor_capture_colors() {
        let syntax = theme::syntax_colors(ThemeChoice::Dark);
        let mut engine = CodeHighlightEngine::new();
        let tokens = engine.line_tokens("fn main() {}\n", LanguageKind::Rust, &syntax);
        assert_eq!(color_at(&tokens, 0, 0), Some(syntax.tokens["keyword"]));
        assert_eq!(color_at(&tokens, 0, 3), Some(syntax.tokens["function"]));
    }

    #[test]
    fn plain_text_gets_no_tokens() {
        let syntax = theme::syntax_colors(ThemeChoice::Dark);
        let mut engine = CodeHighlightEngine::new();
        let tokens = engine.line_tokens("fn main() {}\n", LanguageKind::PlainText, &syntax);
        assert!(tokens.iter().all(Vec::is_empty));
    }

    /// The whole point of the preview highlighter: for the same fence the
    /// editor's injected parse and the preview's standalone parse must produce
    /// byte-identical spans, including the label forms the editor itself
    /// accepts (case-insensitive, `rust,ignore`).
    #[test]
    fn preview_fences_match_the_editor_injection() {
        let syntax = theme::syntax_colors(ThemeChoice::Dark);
        let body = "fn main() {}\n";
        for label in ["rust", "rs", "Rust", "rust,ignore", "ts", "nope"] {
            let markdown = format!("```{label}\n{body}```\n");
            let editor = parse_snapshot(
                &mut Engines::new(),
                &syntax,
                &Rope::from_str(&markdown),
                LanguageKind::Markdown,
                0,
            );
            let mut engine = CodeHighlightEngine::new();
            let preview = engine.line_tokens(body, LanguageKind::from_fence_label(label), &syntax);
            // `PlainText` returns no lines at all; the renderer treats a missing
            // line as foreground, so compare against an empty body line.
            let preview_body = preview.first().cloned().unwrap_or_default();
            assert_eq!(
                editor.line_tokens[1], preview_body,
                "fence ```{label} differs between editor and preview"
            );
        }
    }
}
