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
        // The bundled block query paints the whole `fenced_code_block` as
        // `text.literal` and then clears the body with `@none`. Here `none` is
        // a real foreground capture, so it sits on top of the injected
        // highlights and shadows them. Drop both patterns and let the injected
        // grammar color the code.
        highlights_query: tree_sitter_md::HIGHLIGHT_QUERY_BLOCK
            .replace("  (fenced_code_block)\n] @text.literal", "] @text.literal")
            .replace("(code_fence_content) @none\n", ""),
        // The bundled query omits `injection.include-children` for (inline) and
        // (code_fence_content). Without it the crate reparses only the gaps
        // between zero-width `block_continuation` children as disjoint ranges
        // and the injected grammar misparses.
        injection_query: tree_sitter_md::INJECTION_QUERY_BLOCK
            .replace(
                "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\"))",
                "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\")\n  (#set! injection.include-children))",
            )
            .replace(
                "(code_fence_content) @injection.content)",
                "(code_fence_content) @injection.content\n  (#set! injection.include-children))",
            ),
        locals_query: String::new(),
    }
}
