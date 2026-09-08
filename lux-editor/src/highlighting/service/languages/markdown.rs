use super::def::{ConfigInput, LanguageDef};

pub(super) const DEF: LanguageDef = LanguageDef {
    name: "markdown",
    aliases: &[],
    extensions: &["md", "markdown"],
    requires: &["markdown_inline"],
    config,
};

fn config() -> ConfigInput {
    ConfigInput {
        language: tree_sitter_md::LANGUAGE.into(),
        highlights_query: tree_sitter_md::HIGHLIGHT_QUERY_BLOCK.to_string(),
        // The bundled query omits `injection.include-children` for (inline),
        // so the crate subtracts the emphasis delimiters from the reparsed
        // range and the inline pass would match nothing.
        injection_query: tree_sitter_md::INJECTION_QUERY_BLOCK.replace(
            "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\"))",
            "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\")\n  (#set! injection.include-children))",
        ),
        locals_query: String::new(),
    }
}