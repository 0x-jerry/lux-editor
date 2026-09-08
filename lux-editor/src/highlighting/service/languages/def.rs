use tree_sitter::Language;

/// Pieces built once per process when a language compiles for the first time;
/// queries are owned because a few need runtime concatenation.
pub(crate) struct ConfigInput {
    pub language: Language,
    pub highlights_query: String,
    pub injection_query: String,
    pub locals_query: String,
}

pub(crate) struct LanguageDef {
    /// Canonical injection name (markdown fences, html blocks, …) and the
    /// `language` name passed to `HighlightConfiguration`.
    pub name: &'static str,
    /// Extra injection names that map to this grammar ("sh" → bash).
    pub aliases: &'static [&'static str],
    /// File extensions that select this language.
    pub extensions: &'static [&'static str],
    /// Grammars that must compile together with this one — markdown needs its
    /// inline grammar, which only its injection query can reach.
    pub requires: &'static [&'static str],
    pub config: fn() -> ConfigInput,
}
