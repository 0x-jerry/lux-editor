use super::def::{ConfigInput, LanguageDef};

pub(super) const DEF: LanguageDef = LanguageDef {
    name: "typescript",
    aliases: &["ts"],
    extensions: &["ts", "mts", "cts"],
    requires: &[],
    config,
};

const QUERY: &str = include_str!("../../../../assets/highlights/typescript.scm");

fn config() -> ConfigInput {
    ConfigInput {
        language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        highlights_query: QUERY.to_string(),
        injection_query: String::new(),
        locals_query: String::new(),
    }
}
