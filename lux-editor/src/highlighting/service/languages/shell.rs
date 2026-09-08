use super::def::{ConfigInput, LanguageDef};

pub(super) const DEF: LanguageDef = LanguageDef {
    name: "bash",
    aliases: &["sh", "zsh", "ksh"],
    extensions: &["sh", "bash", "zsh", "ksh"],
    requires: &[],
    config,
};

fn config() -> ConfigInput {
    ConfigInput {
        language: tree_sitter_bash::LANGUAGE.into(),
        highlights_query: tree_sitter_bash::HIGHLIGHT_QUERY.to_string(),
        injection_query: String::new(),
        locals_query: String::new(),
    }
}