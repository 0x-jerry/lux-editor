use super::def::{ConfigInput, LanguageDef};

pub(super) const DEF: LanguageDef = LanguageDef {
    name: "xml",
    aliases: &["svg", "xhtml"],
    extensions: &["xml", "svg", "xhtml"],
    requires: &[],
    config,
};

fn config() -> ConfigInput {
    ConfigInput {
        language: tree_sitter_xml::LANGUAGE_XML.into(),
        highlights_query: tree_sitter_xml::XML_HIGHLIGHT_QUERY.to_string(),
        injection_query: String::new(),
        locals_query: String::new(),
    }
}