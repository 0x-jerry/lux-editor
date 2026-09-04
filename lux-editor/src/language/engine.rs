use tree_sitter::Language;
use tree_sitter_highlight::{
    Error as HighlightError, HighlightConfiguration, HighlightEvent, Highlighter,
};

use super::style::{RECOGNIZED_NAMES, ThemeColors};
use crate::language::LanguageKind;

#[derive(Clone, Copy)]
pub(super) struct RawSpan {
    pub start: usize,
    pub end: usize,
    pub color: [u8; 4],
}

const TYPESCRIPT_QUERY: &str = include_str!("../../assets/highlights/typescript.scm");
const TSX_QUERY: &str = concat!(
    include_str!("../../assets/highlights/typescript.scm"),
    "\n",
    include_str!("../../assets/highlights/typescript-tsx.scm")
);

pub(super) struct Engines {
    highlighter: Highlighter,
    rust: Option<HighlightConfiguration>,
    javascript: Option<HighlightConfiguration>,
    typescript: Option<HighlightConfiguration>,
    tsx: Option<HighlightConfiguration>,
    /// (block, markdown_inline); the inline grammar is reached through the
    /// block query's injection captures.
    markdown: Option<(HighlightConfiguration, HighlightConfiguration)>,
}

fn configure(
    language: Language,
    name: &str,
    highlights_query: &str,
    injection_query: &str,
    locals_query: &str,
) -> Option<HighlightConfiguration> {
    let mut config = HighlightConfiguration::new(
        language,
        name,
        highlights_query,
        injection_query,
        locals_query,
    )
    .inspect_err(|error| log::error!("highlight config for {name} failed: {error}"))
    .ok()?;
    config.configure(RECOGNIZED_NAMES);
    Some(config)
}

impl Engines {
    pub(super) fn new() -> Self {
        let js_query = format!(
            "{}\n{}",
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
        );
        Self {
            highlighter: Highlighter::new(),
            rust: configure(
                tree_sitter_rust::LANGUAGE.into(),
                "rust",
                tree_sitter_rust::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            javascript: configure(
                tree_sitter_javascript::LANGUAGE.into(),
                "javascript",
                &js_query,
                "",
                tree_sitter_javascript::LOCALS_QUERY,
            ),
            typescript: configure(
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                "typescript",
                TYPESCRIPT_QUERY,
                "",
                "",
            ),
            tsx: configure(
                tree_sitter_typescript::LANGUAGE_TSX.into(),
                "tsx",
                TSX_QUERY,
                "",
                "",
            ),
            markdown: configure(
                tree_sitter_md::LANGUAGE.into(),
                "markdown",
                tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
                // The bundled query omits `injection.include-children` for
                // (inline), so the crate subtracts the emphasis delimiters from
                // the reparsed range and the inline pass matches nothing.
                &tree_sitter_md::INJECTION_QUERY_BLOCK.replace(
                    "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\"))",
                    "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\")\n  (#set! injection.include-children))",
                ),
                "",
            )
            .zip(configure(
                tree_sitter_md::INLINE_LANGUAGE.into(),
                "markdown_inline",
                tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
                tree_sitter_md::INJECTION_QUERY_INLINE,
                "",
            )),
        }
    }

    pub(super) fn spans(
        &mut self,
        language: LanguageKind,
        document: &str,
        colors: &ThemeColors,
    ) -> Option<Vec<RawSpan>> {
        let Self {
            highlighter,
            rust,
            javascript,
            typescript,
            tsx,
            markdown,
        } = self;
        let events: Box<dyn Iterator<Item = Result<HighlightEvent, HighlightError>>> =
            match language {
                LanguageKind::Markdown => {
                    let (block, inline) = markdown.as_ref()?;
                    Box::new(
                        highlighter
                            .highlight(block, document.as_bytes(), None, move |name| {
                                (name == "markdown_inline").then_some(inline)
                            })
                            .ok()?,
                    )
                }
                kind => {
                    let config = match kind {
                        LanguageKind::Rust => rust.as_ref()?,
                        LanguageKind::JavaScript => javascript.as_ref()?,
                        LanguageKind::TypeScript => typescript.as_ref()?,
                        LanguageKind::Tsx => tsx.as_ref()?,
                        _ => return Some(Vec::new()),
                    };
                    Box::new(
                        highlighter
                            .highlight(config, document.as_bytes(), None, |_: &str| {
                                Option::<&HighlightConfiguration>::None
                            })
                            .ok()?,
                    )
                }
            };

        let mut spans: Vec<RawSpan> = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        for event in events {
            match event {
                Ok(HighlightEvent::HighlightStart(index)) => stack.push(index.0),
                Ok(HighlightEvent::HighlightEnd) => {
                    stack.pop();
                }
                Ok(HighlightEvent::Source { start, end }) => {
                    if let Some(index) = stack.last().copied() {
                        let color = colors.color(index);
                        if color != colors.foreground && end > start {
                            // Events split at every capture boundary; fuse same-color runs.
                            if let Some(last) = spans.last_mut()
                                && last.color == color
                                && last.end == start
                            {
                                last.end = end;
                                continue;
                            }
                            spans.push(RawSpan { start, end, color });
                        }
                    }
                }
                Err(error) => {
                    log::debug!("highlighting stopped: {error}");
                    break;
                }
            }
        }
        Some(spans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_engine_compiles() {
        let engines = Engines::new();
        assert!(engines.rust.is_some());
        assert!(engines.javascript.is_some());
        assert!(engines.typescript.is_some());
        assert!(engines.tsx.is_some());
        assert!(engines.markdown.is_some());
    }

    #[test]
    fn markdown_inline_injection_is_configured() {
        let engines = Engines::new();
        let (block, _) = engines.markdown.as_ref().unwrap();
        assert!(
            block.query.capture_names().contains(&"injection.content"),
            "block config must carry the inline injection patterns"
        );
    }
}
