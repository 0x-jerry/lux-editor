use crate::language::SyntaxTheme;

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
    pub(super) fn new(theme: SyntaxTheme) -> Self {
        let dark = theme == SyntaxTheme::Dark;
        let foreground = if dark {
            [192, 197, 206, 255]
        } else {
            [79, 91, 102, 255]
        };
        let background = if dark {
            [43, 48, 59, 255]
        } else {
            [239, 241, 245, 255]
        };
        let by_index = RECOGNIZED_NAMES
            .iter()
            .map(|name| color_for(name, dark, foreground))
            .collect();
        Self {
            by_index,
            foreground,
            background,
        }
    }

    pub(super) fn color(&self, index: usize) -> [u8; 4] {
        self.by_index.get(index).copied().unwrap_or(self.foreground)
    }
}

/// Hue families follow the Base16 Ocean palette
/// (the historical default), so dark/light token contrast holds.
fn color_for(name: &str, dark: bool, foreground: [u8; 4]) -> [u8; 4] {
    let (comment, keyword, string, number, function, type_, variable, property, embedded, title) =
        if dark {
            (
                [101, 115, 126, 255],
                [180, 142, 173, 255],
                [163, 190, 140, 255],
                [208, 135, 112, 255],
                [143, 161, 179, 255],
                [235, 203, 139, 255],
                [191, 97, 106, 255],
                [150, 181, 180, 255],
                [208, 135, 112, 255],
                [143, 161, 179, 255],
            )
        } else {
            (
                [167, 173, 186, 255],
                [171, 73, 115, 255],
                [80, 130, 70, 255],
                [170, 90, 50, 255],
                [60, 90, 130, 255],
                [140, 110, 30, 255],
                [159, 59, 59, 255],
                [60, 110, 110, 255],
                [170, 90, 50, 255],
                [60, 90, 130, 255],
            )
        };
    match name {
        "comment" | "comment.documentation" => comment,
        "keyword" => keyword,
        "string" | "text.literal" => string,
        "string.escape" | "escape" | "punctuation.special" => embedded,
        "number" | "constant" | "constant.builtin" | "label" | "text.strong" => number,
        "function" | "function.builtin" | "function.macro" | "function.method" | "constructor"
        | "text.uri" => function,
        "type" | "type.builtin" | "attribute" => type_,
        "variable" | "variable.builtin" | "variable.parameter" | "tag" => variable,
        "property" | "embedded" | "operator" | "text.emphasis" | "text.reference" => property,
        "text.title" => title,
        _ => foreground,
    }
}
