use super::def::{ConfigInput, LanguageDef};

pub(super) const DEF: LanguageDef = LanguageDef {
    name: "json",
    aliases: &["jsonc"],
    // tree-sitter-json is a JSONC grammar: comments are `extras`.
    extensions: &["json", "jsonc"],
    requires: &[],
    config,
};

fn config() -> ConfigInput {
    ConfigInput {
        language: tree_sitter_json::LANGUAGE.into(),
        highlights_query: tree_sitter_json::HIGHLIGHTS_QUERY.to_string(),
        injection_query: String::new(),
        locals_query: String::new(),
    }
}