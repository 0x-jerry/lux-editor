use super::def::{ConfigInput, LanguageDef};

pub(super) const DEF: LanguageDef = LanguageDef {
    name: "yaml",
    aliases: &["yml"],
    extensions: &["yaml", "yml"],
    requires: &[],
    config,
};

fn config() -> ConfigInput {
    ConfigInput {
        language: tree_sitter_yaml::LANGUAGE.into(),
        highlights_query: tree_sitter_yaml::HIGHLIGHTS_QUERY.to_string(),
        injection_query: String::new(),
        locals_query: String::new(),
    }
}