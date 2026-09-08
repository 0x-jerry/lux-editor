use super::def::{ConfigInput, LanguageDef};

/// Injection-only grammar: no file extension selects it and it has no
/// `LanguageKind` variant; markdown reaches it by name.
pub(super) const DEF: LanguageDef = LanguageDef {
    name: "markdown_inline",
    aliases: &[],
    extensions: &[],
    requires: &[],
    config,
};

fn config() -> ConfigInput {
    ConfigInput {
        language: tree_sitter_md::INLINE_LANGUAGE.into(),
        highlights_query: tree_sitter_md::HIGHLIGHT_QUERY_INLINE.to_string(),
        injection_query: tree_sitter_md::INJECTION_QUERY_INLINE.to_string(),
        locals_query: String::new(),
    }
}