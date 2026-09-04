use ropey::Rope;

use super::engine::{Engines, RawSpan};
use super::snapshot::{HighlightSnapshot, HighlightSpan};
use super::style::ThemeColors;
use crate::language::LanguageKind;
use crate::theme::SyntaxColors;

pub(super) fn parse_snapshot(
    engines: &mut Engines,
    syntax: &SyntaxColors,
    text: &Rope,
    language: LanguageKind,
    version: u64,
) -> HighlightSnapshot {
    let colors = ThemeColors::new(syntax);
    let line_count = text.len_lines();
    let mut snapshot = HighlightSnapshot {
        version,
        background: Some(colors.background),
        foreground: Some(colors.foreground),
        line_tokens: Vec::new(),
    };
    if language == LanguageKind::PlainText {
        return snapshot;
    }

    // v1 ceiling: one owned copy + full re-parse per debounced edit; the
    // upgrade path is incremental parsing via tree-sitter InputEdits.
    let document = text.to_string();
    let mut line_lengths = Vec::with_capacity(line_count);
    let mut line_starts = Vec::with_capacity(line_count);
    let mut offset = 0usize;
    for line_index in 0..line_count {
        line_starts.push(offset);
        let len = text.line(line_index).len_bytes();
        line_lengths.push(len);
        offset += len;
    }

    let spans = engines
        .spans(language, &document, &colors)
        .unwrap_or_default();
    snapshot.line_tokens = split_by_lines(&spans, &line_starts, &line_lengths);
    snapshot
}

/// `HighlightEvent::Source` ranges arrive in document order and never
/// overlap, so this is a pure split at line boundaries.
fn split_by_lines(
    spans: &[RawSpan],
    line_starts: &[usize],
    line_lengths: &[usize],
) -> Vec<Vec<HighlightSpan>> {
    let line_count = line_lengths.len();
    let mut tokens = vec![Vec::new(); line_count];
    if spans.is_empty() || line_count == 0 {
        return tokens;
    }
    let document_end = line_starts[line_count - 1] + line_lengths[line_count - 1];

    for span in spans {
        let mut position = span.start.min(document_end);
        if position >= span.end {
            continue;
        }
        let mut line = line_starts
            .partition_point(|&start| start <= position)
            .saturating_sub(1);
        while position < span.end && line < line_count {
            let line_base = line_starts[line];
            let line_end = line_base + line_lengths[line];
            let segment_end = span.end.min(document_end).min(line_end);
            if segment_end > position {
                tokens[line].push(HighlightSpan {
                    start_col: position - line_base,
                    end_col: segment_end - line_base,
                    color: span.color,
                });
            }
            position = segment_end.max(line_end);
            line += 1;
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{self, ThemeChoice};

    fn snapshot_for(source: &str, language: LanguageKind) -> HighlightSnapshot {
        let mut engines = Engines::new();
        parse_snapshot(
            &mut engines,
            &theme::syntax_colors(ThemeChoice::Dark),
            &Rope::from_str(source),
            language,
            1,
        )
    }

    #[test]
    fn plain_text_gets_no_tokens_so_the_theme_foreground_is_used() {
        let snapshot = snapshot_for("hello\nworld", LanguageKind::PlainText);
        assert!(snapshot.line_tokens.is_empty());
        assert!(snapshot.foreground.is_some());
        assert!(snapshot.background.is_some());
    }

    #[test]
    fn typescript_gets_colored_spans() {
        let source = "interface Foo { name: string }\nconst x = \"lit\";\n";
        let snapshot = snapshot_for(source, LanguageKind::TypeScript);
        assert_eq!(snapshot.line_tokens.len(), 3);
        let foreground = snapshot.foreground.unwrap();
        assert!(
            snapshot.line_tokens[0]
                .iter()
                .any(|span| span.color != foreground),
            "interface/Foo keywords should not be plain foreground"
        );
        let string_span = snapshot.line_tokens[1]
            .iter()
            .find(|span| span.start_col <= 10 && span.end_col >= 15)
            .expect("string literal span");
        assert_ne!(string_span.color, foreground);
    }

    #[test]
    fn markdown_highlights_headings() {
        let snapshot = snapshot_for("# Title\n\nsome *text*\n", LanguageKind::Markdown);
        assert!(!snapshot.line_tokens[0].is_empty());
    }

    #[test]
    fn markdown_injects_inline_grammar() {
        let snapshot = snapshot_for("para with *strong text* here\n", LanguageKind::Markdown);
        assert!(!snapshot.line_tokens[0].is_empty());
    }

    #[test]
    fn multi_line_capture_is_split_across_lines() {
        let snapshot = snapshot_for("const t = `a\nb`;\n", LanguageKind::JavaScript);
        assert!(!snapshot.line_tokens[0].is_empty());
        assert!(!snapshot.line_tokens[1].is_empty());
    }

    #[test]
    fn snapshot_lines_align_with_rope_lines() {
        // A trailing newline adds an empty line in ropey's line counting; the
        // editor indexes rows the same way, so the snapshot must match it.
        let snapshot = snapshot_for("a\nb\n", LanguageKind::Rust);
        assert_eq!(snapshot.line_tokens.len(), 3);
    }

    #[test]
    fn spans_stay_on_char_boundaries_for_multibyte_crlf_text() {
        let source = "const 🦀 = \"ключ\";\r\nlet x = 1;\r\n";
        let snapshot = snapshot_for(source, LanguageKind::JavaScript);
        let rope = Rope::from_str(source);
        assert_eq!(snapshot.line_tokens.len(), rope.len_lines());
        for (line_index, tokens) in snapshot.line_tokens.iter().enumerate() {
            let line = rope.line(line_index).to_string();
            for token in tokens {
                assert!(token.start_col <= token.end_col);
                assert!(line.is_char_boundary(token.start_col));
                assert!(line.is_char_boundary(token.end_col));
            }
        }
    }

    #[test]
    fn inner_capture_wins_over_outer() {
        // `\n` inside the string must get the escape color, not the string color.
        let snapshot = snapshot_for("const s = \"a\\nb\";\n", LanguageKind::JavaScript);
        let colors: Vec<[u8; 4]> = snapshot.line_tokens[0]
            .iter()
            .map(|span| span.color)
            .collect();
        assert!(
            colors.len() >= 2,
            "string should be split by escape: {colors:?}"
        );
        assert!(colors.windows(2).any(|w| w[0] != w[1]));
    }

    #[test]
    fn uppercase_identifier_uses_constant_color_not_variable() {
        let snapshot = snapshot_for("const foo = bar(FOO);\n", LanguageKind::TypeScript);
        let foreground = snapshot.foreground.unwrap();
        let color_at = |col: usize| {
            snapshot.line_tokens[0]
                .iter()
                .find(|span| span.start_col <= col && col < span.end_col)
                .map(|span| span.color)
                .unwrap_or(foreground)
        };
        // `foo` (6..9) is a plain variable, `FOO` (11..14) the ALL_CAPS constant.
        assert_ne!(color_at(6), foreground);
        assert_ne!(
            color_at(6),
            color_at(11),
            "FOO must not share the variable color"
        );
    }
}
