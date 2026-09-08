use super::def::{ConfigInput, LanguageDef};

pub(super) const DEF: LanguageDef = LanguageDef {
    name: "javascript",
    aliases: &["js", "mjs", "cjs", "jsx"],
    extensions: &["js", "mjs", "cjs", "jsx"],
    requires: &[],
    config,
};

fn config() -> ConfigInput {
    ConfigInput {
        language: tree_sitter_javascript::LANGUAGE.into(),
        highlights_query: format!(
            "{}\n{}",
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
        ),
        injection_query: String::new(),
        locals_query: tree_sitter_javascript::LOCALS_QUERY.to_string(),
    }
}