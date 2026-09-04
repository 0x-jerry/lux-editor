use tree_sitter::Language;
use tree_sitter_highlight::{
    Error as HighlightError, HighlightConfiguration, HighlightEvent, Highlighter,
};

use super::style::{RECOGNIZED_NAMES, ThemeColors};
use super::LanguageKind;
use std::sync::LazyLock;

#[derive(Clone, Copy)]
pub(super) struct RawSpan {
    pub start: usize,
    pub end: usize,
    pub color: [u8; 4],
}

const TYPESCRIPT_QUERY: &str = include_str!("../../../assets/highlights/typescript.scm");
const TSX_QUERY: &str = concat!(
    include_str!("../../../assets/highlights/typescript.scm"),
    "\n",
    include_str!("../../../assets/highlights/typescript-tsx.scm")
);
static JS_QUERY: LazyLock<String> = LazyLock::new(|| {
    format!(
        "{}\n{}",
        tree_sitter_javascript::HIGHLIGHT_QUERY,
        tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
    )
});
// The bundled query omits `injection.include-children` for (inline), so the
// crate subtracts the emphasis delimiters from the reparsed range and the
// inline pass matches nothing.
static MD_INJECTION_QUERY: LazyLock<String> = LazyLock::new(|| {
    tree_sitter_md::INJECTION_QUERY_BLOCK.replace(
        "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\"))",
        "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\")\n  (#set! injection.include-children))",
    )
});

/// One `HighlightConfiguration` per language, compiled on the first request
/// for that language only — thread startup costs nothing at launch.
#[derive(Default)]
pub(super) struct Engines {
    highlighter: Highlighter,
    /// Bit per tried language: a `configure` failure must not make every
    /// subsequent parse retry the compile (and re-log the error).
    tried: u8,
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
        Self::default()
    }

    fn ensure(&mut self, kind: LanguageKind) {
        let bit = match kind {
            LanguageKind::Rust => 1 << 0,
            LanguageKind::JavaScript => 1 << 1,
            LanguageKind::TypeScript => 1 << 2,
            LanguageKind::Tsx => 1 << 3,
            LanguageKind::Markdown => 1 << 4,
            LanguageKind::PlainText => return,
        };
        if self.tried & bit != 0 {
            return;
        }
        self.tried |= bit;
        match kind {
            LanguageKind::Rust if self.rust.is_none() => {
                self.rust = configure(
                    tree_sitter_rust::LANGUAGE.into(),
                    "rust",
                    tree_sitter_rust::HIGHLIGHTS_QUERY,
                    "",
                    "",
                );
            }
            LanguageKind::JavaScript if self.javascript.is_none() => {
                self.javascript = configure(
                    tree_sitter_javascript::LANGUAGE.into(),
                    "javascript",
                    &JS_QUERY,
                    "",
                    tree_sitter_javascript::LOCALS_QUERY,
                );
            }
            LanguageKind::TypeScript if self.typescript.is_none() => {
                self.typescript = configure(
                    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                    "typescript",
                    TYPESCRIPT_QUERY,
                    "",
                    "",
                );
            }
            LanguageKind::Tsx if self.tsx.is_none() => {
                self.tsx = configure(
                    tree_sitter_typescript::LANGUAGE_TSX.into(),
                    "tsx",
                    TSX_QUERY,
                    "",
                    "",
                );
            }
            LanguageKind::Markdown if self.markdown.is_none() => {
                self.markdown = configure(
                    tree_sitter_md::LANGUAGE.into(),
                    "markdown",
                    tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
                    &MD_INJECTION_QUERY,
                    "",
                )
                .zip(configure(
                    tree_sitter_md::INLINE_LANGUAGE.into(),
                    "markdown_inline",
                    tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
                    tree_sitter_md::INJECTION_QUERY_INLINE,
                    "",
                ));
            }
            _ => {}
        }
    }

    pub(super) fn spans(
        &mut self,
        language: LanguageKind,
        document: &str,
        colors: &ThemeColors,
    ) -> Option<Vec<RawSpan>> {
        self.ensure(language);
        let Self {
            highlighter,
            rust,
            javascript,
            typescript,
            tsx,
            markdown,
            ..
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
        let mut engines = Engines::new();
        for kind in [
            LanguageKind::Rust,
            LanguageKind::JavaScript,
            LanguageKind::TypeScript,
            LanguageKind::Tsx,
            LanguageKind::Markdown,
        ] {
            engines.ensure(kind);
        }
        assert!(engines.rust.is_some());
        assert!(engines.javascript.is_some());
        assert!(engines.typescript.is_some());
        assert!(engines.tsx.is_some());
        assert!(engines.markdown.is_some());
    }

    #[test]
    fn markdown_inline_injection_is_configured() {
        let mut engines = Engines::new();
        engines.ensure(LanguageKind::Markdown);
        let (block, _) = engines.markdown.as_ref().unwrap();
        assert!(
            block.query.capture_names().contains(&"injection.content"),
            "block config must carry the inline injection patterns"
        );
    }
}
