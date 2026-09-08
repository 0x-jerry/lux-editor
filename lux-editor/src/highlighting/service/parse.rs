use ropey::Rope;

use super::LanguageKind;
use super::engine::{Engines, RawSpan};
use super::snapshot::{HighlightSnapshot, HighlightSpan};
use super::style::ThemeColors;
use crate::highlighting::paint::clip_to_cursor;
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
/// overlap, so this is a pure split at line boundaries, followed by a
/// normalization pass that guarantees each line's tokens are disjoint and
/// column-ordered (see [`normalize_line_tokens`]).
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
    for line_tokens in tokens.iter_mut() {
        normalize_line_tokens(line_tokens);
    }
    tokens
}

/// Enforces the `line_tokens` contract on a single line: tokens are sorted by
/// column and clipped against a monotonic cursor so they never overlap
/// (zero-width residuals are dropped). It shares [`clip_to_cursor`] with the
/// painter, so both layers use the identical "first token wins a shared column"
/// rule and cannot drift. Note that rule is a local convention for deterministic
/// output, NOT tree-sitter's inner-capture-wins semantics — the engine already
/// bakes inner-capture-wins into disjoint segments, so this only matters for
/// hypothetical upstream overlap. Defense-in-depth: the painter re-clips anyway.
fn normalize_line_tokens(tokens: &mut Vec<HighlightSpan>) {
    if tokens.len() < 2 {
        return;
    }
    tokens.sort_by_key(|t| (t.start_col, t.end_col));
    let mut write = 0usize;
    let mut cursor = 0usize;
    for read in 0..tokens.len() {
        let token = tokens[read];
        if let Some((start, end)) = clip_to_cursor(token.start_col, token.end_col, cursor) {
            tokens[write] = HighlightSpan {
                start_col: start,
                end_col: end,
                color: token.color,
            };
            write += 1;
            cursor = cursor.max(end);
        }
    }
    tokens.truncate(write);
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

    fn color_at(snapshot: &HighlightSnapshot, line: usize, col: usize) -> [u8; 4] {
        let foreground = snapshot.foreground.unwrap();
        snapshot.line_tokens[line]
            .iter()
            .find(|span| span.start_col <= col && col < span.end_col)
            .map(|span| span.color)
            .unwrap_or(foreground)
    }

    #[test]
    fn split_by_lines_overlaps_are_normalized() {
        // Simulate an upstream grammar emitting overlapping ranges on one line.
        let spans = vec![
            RawSpan { start: 0, end: 4, color: [1, 2, 3, 255] },
            RawSpan { start: 2, end: 8, color: [4, 5, 6, 255] },
            RawSpan { start: 6, end: 10, color: [7, 8, 9, 255] },
        ];
        // One line of 10 bytes.
        let starts = vec![0];
        let lengths = vec![10];
        let tokens = split_by_lines(&spans, &starts, &lengths);
        let tokens = &tokens[0];

        // Disjoint, ordered, cover 0..10 exactly, no duplicates.
        let mut cursor = 0usize;
        for t in tokens {
            assert!(t.start_col >= cursor, "token must not overlap the previous: {t:?}");
            assert!(t.start_col < t.end_col, "zero-width token: {t:?}");
            assert_eq!(t.start_col, cursor, "token should start where the previous left off: {t:?}");
            cursor = t.end_col;
        }
        assert_eq!(cursor, 10, "tokens must cover the whole line");
    }

    #[test]
    fn markdown_fence_closing_lines_are_not_duplicated_when_painted() {
        // Mirrors row.rs: trim the trailing newline (display_line_text), then
        // build the LayoutJob. A closing fence `` ``` `` must paint exactly 3
        // backticks, not 6. This is the end-to-end regression for the phantom
        // backtick bug.
        use crate::highlighting::build_highlighted_line_job;
        use eframe::egui;

        let source = "# Title\n\n```ts\nconsole.log(1)\n```\n\n```rs\nfn main() {}\n```\n";
        let snapshot = snapshot_for(source, LanguageKind::Markdown);
        let rope = Rope::from_str(source);
        for line_index in 0..snapshot.line_tokens.len() {
            let raw = rope.line(line_index).to_string();
            let trimmed = raw.trim_end_matches(['\r', '\n']);
            let tokens = snapshot.line_tokens.get(line_index).map(Vec::as_slice).unwrap_or(&[]);
            let job = build_highlighted_line_job(
                trimmed,
                tokens,
                12.0,
                egui::Color32::GRAY,
            );
            assert_eq!(
                job.text, trimmed,
                "line {line_index} painted output must equal the trimmed source (no duplication)"
            );
        }
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
    fn markdown_injects_registered_fence_languages() {
        let snapshot = snapshot_for(
            "# Title\n\n```rust\nfn main() {}\n```\n\n```sh\necho \"hi\" # comment\n```\n\n```nope\nplain body\n```\n",
            LanguageKind::Markdown,
        );
        let foreground = snapshot.foreground.unwrap();
        let syntax = theme::syntax_colors(ThemeChoice::Dark);
        // Lines 3 and 7 are the ```rust/```sh fence bodies; line 11, a fence
        // no registered grammar matches, stays plain.
        for body_index in [3, 7] {
            assert!(
                snapshot.line_tokens[body_index]
                    .iter()
                    .any(|span| span.color != foreground),
                "line {body_index} must be highlighted via the registered fence language"
            );
        }
        assert!(snapshot.line_tokens[11].is_empty());
        // The injected spans must land on their real columns, not smear across
        // the line: `fn` keyword and `main` function.
        assert_eq!(color_at(&snapshot, 3, 0), syntax.tokens["keyword"]);
        assert_eq!(color_at(&snapshot, 3, 3), syntax.tokens["function"]);
    }

    #[test]
    fn markdown_fence_bodies_keep_injected_offsets() {
        // Mirrors playground/test.md: a misaligned injected parse used to drop
        // `fn` and paint whole lines with one capture's color.
        let snapshot = snapshot_for(
            "```ts\nconsole.log('hello')\n```\n\n```rs\nfn main() {\n    println!(\"hello\");\n}\n```\n",
            LanguageKind::Markdown,
        );
        let syntax = theme::syntax_colors(ThemeChoice::Dark);
        assert_eq!(color_at(&snapshot, 1, 0), syntax.tokens["variable"], "console");
        assert_eq!(color_at(&snapshot, 1, 8), syntax.tokens["function"], "log");
        assert_eq!(color_at(&snapshot, 1, 12), syntax.tokens["string"], "'hello'");
        assert_eq!(color_at(&snapshot, 5, 0), syntax.tokens["keyword"], "fn");
        assert_eq!(color_at(&snapshot, 5, 3), syntax.tokens["function"], "main");
        assert_eq!(color_at(&snapshot, 6, 13), syntax.tokens["string"], "\"hello\"");
    }

    #[test]
    fn markdown_injects_frontmatter() {
        let snapshot = snapshot_for("---\nflag: true\n---\n\n# Body\n", LanguageKind::Markdown);
        let foreground = snapshot.foreground.unwrap();
        assert!(
            snapshot.line_tokens[1]
                .iter()
                .any(|span| span.color != foreground),
            "frontmatter must be parsed with the yaml grammar"
        );
    }

    #[test]
    fn json_yaml_toml_shell_get_colored_spans() {
        let syntax = theme::syntax_colors(ThemeChoice::Dark);
        let boolean_color = syntax.tokens["boolean"];
        for (source, kind, expect) in [
            ("{\"key\": true}\n", LanguageKind::Json, None),
            ("key: value\n# comment\n", LanguageKind::Yaml, None),
            (
                "[section]\nflag = true\n",
                LanguageKind::Toml,
                Some(boolean_color),
            ),
        ] {
            let snapshot = snapshot_for(source, kind);
            let colored: Vec<[u8; 4]> = snapshot
                .line_tokens
                .iter()
                .flatten()
                .map(|s| s.color)
                .collect();
            assert!(
                colored.iter().any(|c| *c != snapshot.foreground.unwrap()),
                "{kind:?}: {colored:?}"
            );
            if let Some(expected) = expect {
                assert!(
                    snapshot.line_tokens[1].iter().any(|s| s.color == expected),
                    "{kind:?} boolean must use the boolean color"
                );
            }
        }
        let snapshot = snapshot_for("echo \"hi\" # comment\n", LanguageKind::Shell);
        assert!(
            snapshot.line_tokens[0]
                .iter()
                .any(|span| span.color != snapshot.foreground.unwrap()),
        );
    }

    #[test]
    fn json_comments_are_colored() {
        // .jsonc is the same engine: the grammar treats comments as `extras`.
        let comment_color = theme::syntax_colors(ThemeChoice::Dark).tokens["comment"];
        let snapshot = snapshot_for("{\n  // note\n  \"a\": 1\n}\n", LanguageKind::Json);
        assert!(
            snapshot.line_tokens[1]
                .iter()
                .any(|span| span.color == comment_color),
            "line comment must use the comment color"
        );
    }

    #[test]
    fn xml_and_html_get_colored_spans() {
        let comment_color = theme::syntax_colors(ThemeChoice::Dark).tokens["comment"];
        for (source, kind) in [
            (
                "<?xml version=\"1.0\"?>\n<note>\n  <to>Tove</to>\n</note>\n",
                LanguageKind::Xml,
            ),
            ("<div class=\"box\"><p>Text</p></div>\n", LanguageKind::Html),
        ] {
            let snapshot = snapshot_for(source, kind);
            let colored: Vec<[u8; 4]> = snapshot
                .line_tokens
                .iter()
                .flatten()
                .map(|s| s.color)
                .collect();
            assert!(
                colored.iter().any(|c| *c != snapshot.foreground.unwrap()),
                "{kind:?} should color tag/attribute spans: {colored:?}"
            );
        }
        let snapshot = snapshot_for("<!-- hi -->\n", LanguageKind::Html);
        assert!(
            snapshot.line_tokens[0]
                .iter()
                .any(|span| span.color == comment_color),
            "html comment must use the comment color"
        );
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
        // `foo` (6..9) is a plain variable, `FOO` (11..14) the ALL_CAPS constant.
        assert_ne!(color_at(&snapshot, 0, 6), foreground);
        assert_ne!(
            color_at(&snapshot, 0, 6),
            color_at(&snapshot, 0, 11),
            "FOO must not share the variable color"
        );
    }
}
