use crate::highlighting::{CodeHighlightEngine, HighlightSpan, LanguageKind};
use crate::theme::SyntaxColors;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Memoizes tree-sitter token runs for preview fences. The preview re-renders
/// every frame, so `begin_frame` drops the entries no fence used last frame and
/// keeps the rest: a static document parses each fence once, while memory stays
/// bounded by the live fence count rather than by a fixed cap (a cap below the
/// fence count would clear the map every frame and re-parse everything).
#[derive(Default)]
pub(crate) struct CodeHighlighter {
    engine: CodeHighlightEngine,
    cache: HashMap<CodeKey, Entry>,
    generation: u64,
    #[cfg(test)]
    parses: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct CodeKey {
    language: LanguageKind,
    /// `Arc::as_ptr` identity, so a theme change invalidates every entry.
    palette: usize,
    text: u64,
}

struct Entry {
    text: String,
    tokens: Arc<Vec<Vec<HighlightSpan>>>,
    generation: u64,
}

impl CodeHighlighter {
    /// Drops entries no fence used since the previous call, then opens a new
    /// generation. Call once per rendered frame.
    pub(crate) fn begin_frame(&mut self) {
        let current = self.generation;
        self.cache.retain(|_, entry| entry.generation == current);
        self.generation = self.generation.wrapping_add(1);
    }

    pub(crate) fn line_tokens(
        &mut self,
        code: &str,
        language: LanguageKind,
        syntax: &Arc<SyntaxColors>,
    ) -> Arc<Vec<Vec<HighlightSpan>>> {
        let key = CodeKey {
            language,
            palette: Arc::as_ptr(syntax) as usize,
            text: hash_text(code),
        };
        // The hash can collide; compare the text before trusting a hit.
        if let Some(entry) = self.cache.get_mut(&key)
            && entry.text == code
        {
            entry.generation = self.generation;
            return Arc::clone(&entry.tokens);
        }
        #[cfg(test)]
        {
            self.parses += 1;
        }
        let tokens = Arc::new(self.engine.line_tokens(code, language, syntax));
        self.cache.insert(
            key,
            Entry {
                text: code.to_owned(),
                tokens: Arc::clone(&tokens),
                generation: self.generation,
            },
        );
        tokens
    }
}

fn hash_text(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{self, ThemeChoice};

    fn colors(
        highlighter: &mut CodeHighlighter,
        code: &str,
        language: LanguageKind,
        syntax: &Arc<SyntaxColors>,
    ) -> Vec<[u8; 4]> {
        highlighter
            .line_tokens(code, language, syntax)
            .iter()
            .flatten()
            .map(|span| span.color)
            .collect()
    }

    #[test]
    fn a_fence_uses_the_theme_palette() {
        let syntax = theme::syntax_colors(ThemeChoice::Dark);
        let mut highlighter = CodeHighlighter::default();
        let colors = colors(
            &mut highlighter,
            "fn main() {}",
            LanguageKind::Rust,
            &syntax,
        );
        assert!(colors.contains(&syntax.tokens["keyword"]));
        assert!(colors.contains(&syntax.tokens["function"]));
    }

    #[test]
    fn a_palette_change_recomputes_instead_of_serving_the_cache() {
        let dark = theme::syntax_colors(ThemeChoice::Dark);
        let light = theme::syntax_colors(ThemeChoice::Light);
        assert_ne!(dark.tokens["keyword"], light.tokens["keyword"]);
        let mut highlighter = CodeHighlighter::default();
        let dark_colors = colors(&mut highlighter, "fn main() {}", LanguageKind::Rust, &dark);
        let light_colors = colors(&mut highlighter, "fn main() {}", LanguageKind::Rust, &light);
        assert!(dark_colors.contains(&dark.tokens["keyword"]));
        assert!(light_colors.contains(&light.tokens["keyword"]));
    }

    #[test]
    fn plain_text_has_no_token_runs() {
        let syntax = theme::syntax_colors(ThemeChoice::Dark);
        let mut highlighter = CodeHighlighter::default();
        assert!(
            colors(
                &mut highlighter,
                "fn main() {}",
                LanguageKind::PlainText,
                &syntax
            )
            .is_empty()
        );
    }

    #[test]
    fn a_static_document_parses_each_fence_once() {
        let syntax = theme::syntax_colors(ThemeChoice::Dark);
        let mut highlighter = CodeHighlighter::default();
        let codes: Vec<String> = (0..70).map(|i| format!("fn f{i}() {{}}")).collect();

        highlighter.begin_frame();
        for code in &codes {
            highlighter.line_tokens(code, LanguageKind::Rust, &syntax);
        }
        assert_eq!(highlighter.parses, codes.len());

        highlighter.begin_frame();
        for code in &codes {
            highlighter.line_tokens(code, LanguageKind::Rust, &syntax);
        }
        assert_eq!(
            highlighter.parses,
            codes.len(),
            "fences that did not change must hit the cache"
        );
    }

    #[test]
    fn entries_unused_for_a_frame_are_evicted() {
        let syntax = theme::syntax_colors(ThemeChoice::Dark);
        let mut highlighter = CodeHighlighter::default();

        highlighter.begin_frame();
        highlighter.line_tokens("fn a() {}", LanguageKind::Rust, &syntax);
        highlighter.begin_frame();
        highlighter.line_tokens("fn b() {}", LanguageKind::Rust, &syntax);
        assert_eq!(highlighter.cache.len(), 2);

        highlighter.begin_frame();
        assert_eq!(
            highlighter.cache.len(),
            1,
            "the unused fence should be gone"
        );
    }
}
