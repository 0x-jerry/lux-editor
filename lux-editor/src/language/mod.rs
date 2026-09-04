mod engine;
mod parse;
mod service;
mod snapshot;
mod style;
mod worker;

pub use service::HighlightingService;
pub use snapshot::{HighlightSnapshot, HighlightSpan, SyntaxTheme};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageKind {
    PlainText,
    Rust,
    JavaScript,
    TypeScript,
    Tsx,
    Markdown,
}

impl LanguageKind {
    fn from_extension(extension: &str) -> Self {
        match extension {
            "rs" => Self::Rust,
            "js" | "mjs" | "cjs" | "jsx" => Self::JavaScript,
            "ts" | "mts" | "cts" => Self::TypeScript,
            "tsx" => Self::Tsx,
            "md" | "markdown" => Self::Markdown,
            _ => Self::PlainText,
        }
    }

    pub fn from_path(path: Option<&std::path::Path>) -> Self {
        let Some(path) = path else {
            return Self::PlainText;
        };
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| Self::from_extension(&ext.to_ascii_lowercase()))
            .unwrap_or(Self::PlainText)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn language_kind_from_path() {
        assert_eq!(LanguageKind::from_path(None), LanguageKind::PlainText);
        assert_eq!(
            LanguageKind::from_path(Some(Path::new("main.rs"))),
            LanguageKind::Rust
        );
        assert_eq!(
            LanguageKind::from_path(Some(Path::new("FILE.TS"))),
            LanguageKind::TypeScript
        );
        assert_eq!(
            LanguageKind::from_path(Some(Path::new("app.tsx"))),
            LanguageKind::Tsx
        );
        assert_eq!(
            LanguageKind::from_path(Some(Path::new("index.mjs"))),
            LanguageKind::JavaScript
        );
        assert_eq!(
            LanguageKind::from_path(Some(Path::new("README.md"))),
            LanguageKind::Markdown
        );
        assert_eq!(
            LanguageKind::from_path(Some(Path::new("README"))),
            LanguageKind::PlainText
        );
        assert_eq!(
            LanguageKind::from_path(Some(Path::new("weird.zzz"))),
            LanguageKind::PlainText
        );
    }
}
