use super::def::{ConfigInput, LanguageDef};

pub(super) const DEF: LanguageDef = LanguageDef {
    name: "python",
    aliases: &["py"],
    extensions: &["py"],
    requires: &[],
    config,
};

fn config() -> ConfigInput {
    ConfigInput {
        language: tree_sitter_python::LANGUAGE.into(),
        highlights_query: tree_sitter_python::HIGHLIGHTS_QUERY.to_string(),
        injection_query: String::new(),
        locals_query: String::new(),
    }
}
