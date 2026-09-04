#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyntaxTheme {
    Dark,
    Light,
}

impl SyntaxTheme {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HighlightSpan {
    pub start_col: usize,
    pub end_col: usize,
    pub color: [u8; 4],
}

#[derive(Clone, Debug, Default)]
pub struct HighlightSnapshot {
    pub version: u64,
    /// Built-in theme background, used for the editor chrome.
    pub background: Option<[u8; 4]>,
    /// Built-in theme foreground, used for plain text tokens.
    pub foreground: Option<[u8; 4]>,
    pub line_tokens: Vec<Vec<HighlightSpan>>,
}
