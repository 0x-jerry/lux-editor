use super::def::{ConfigInput, LanguageDef};

pub(super) const DEF: LanguageDef = LanguageDef {
    name: "tsx",
    aliases: &[],
    extensions: &["tsx"],
    requires: &[],
    config,
};

const QUERY: &str = concat!(
    include_str!("../../../../assets/highlights/typescript.scm"),
    "\n",
    include_str!("../../../../assets/highlights/typescript-tsx.scm")
);

fn config() -> ConfigInput {
    ConfigInput {
        language: tree_sitter_typescript::LANGUAGE_TSX.into(),
        highlights_query: QUERY.to_string(),
        injection_query: String::new(),
        locals_query: String::new(),
    }
}