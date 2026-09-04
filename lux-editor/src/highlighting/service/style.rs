use crate::theme::SyntaxColors;

/// Highlight names recognized by every `HighlightConfiguration`; the index a
/// `Highlight` carries refers to this list. Superset of the capture names in
/// the bundled queries plus the tree-sitter standard names.
pub(super) static RECOGNIZED_NAMES: &[&str] = &[
    "attribute",
    "comment",
    "comment.documentation",
    "constant",
    "constant.builtin",
    "constructor",
    "embedded",
    "escape",
    "function",
    "function.builtin",
    "function.macro",
    "function.method",
    "keyword",
    "label",
    "none",
    "number",
    "operator",
    "property",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    "string",
    "string.escape",
    "string.special",
    "tag",
    "text.emphasis",
    "text.literal",
    "text.reference",
    "text.strong",
    "text.title",
    "text.uri",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
];

/// Pre-resolved color per `Highlight` index for one theme, so the hot path is
/// a single indexing step.
pub(super) struct ThemeColors {
    by_index: Vec<[u8; 4]>,
    pub foreground: [u8; 4],
    pub background: [u8; 4],
}

impl ThemeColors {
    pub(super) fn new(syntax: &SyntaxColors) -> Self {
        let by_index = RECOGNIZED_NAMES
            .iter()
            .map(|name| {
                syntax
                    .tokens
                    .get(*name)
                    .copied()
                    .unwrap_or(syntax.foreground)
            })
            .collect();
        Self {
            by_index,
            foreground: syntax.foreground,
            background: syntax.background,
        }
    }

    pub(super) fn color(&self, index: usize) -> [u8; 4] {
        self.by_index.get(index).copied().unwrap_or(self.foreground)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{self, ThemeChoice};

    #[test]
    fn builtin_token_names_are_recognized() {
        // A typo in the theme JSON would silently fall back to foreground.
        for choice in [ThemeChoice::Dark, ThemeChoice::Light] {
            for name in theme::syntax_colors(choice).tokens.keys() {
                assert!(
                    RECOGNIZED_NAMES.contains(&name.as_str()),
                    "{name} is not a recognized highlight name"
                );
            }
        }
    }

    #[test]
    fn unlisted_tokens_fall_back_to_foreground() {
        let colors = ThemeColors::new(&theme::syntax_colors(ThemeChoice::Dark));
        let none_index = RECOGNIZED_NAMES.iter().position(|n| *n == "none").unwrap();
        assert_eq!(colors.color(none_index), colors.foreground);
        assert_eq!(colors.color(9999), colors.foreground);
    }
}
