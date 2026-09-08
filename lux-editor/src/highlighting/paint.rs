use crate::highlighting::HighlightSpan;
use eframe::egui;

/// Converts an optional rgba color from the highlight worker into a `Color32`,
/// falling back on the chrome-provided color when the theme is silent.
pub fn snapshot_color(color: Option<[u8; 4]>, fallback: egui::Color32) -> egui::Color32 {
    color
        .map(|[r, g, b, a]| egui::Color32::from_rgba_unmultiplied(r, g, b, a))
        .unwrap_or(fallback)
}

pub fn build_highlighted_line_job(
    line: &str,
    tokens: &[HighlightSpan],
    font_size: f32,
    default_color: egui::Color32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    if tokens.is_empty() {
        job.append(
            line,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(font_size),
                color: default_color,
                ..Default::default()
            },
        );
        return job;
    }

    let mut cursor = 0usize;
    for token in tokens {
        // Token offsets are byte-based; snap to UTF-8 boundaries so slicing can't panic.
        let mut start = token.start_col.min(line.len());
        let mut end = token.end_col.min(line.len());
        while start > 0 && !line.is_char_boundary(start) {
            start -= 1;
        }
        while end < line.len() && !line.is_char_boundary(end) {
            end += 1;
        }
        // Paint only the not-yet-painted sub-range (first token wins a shared
        // column). This both keeps overlapping tokens from re-emitting the same
        // glyphs (e.g. a fence `` ``` `` painted twice) and, by advancing
        // `cursor` over the painted gap, stops the trailing append below from
        // duplicating the line when a token lies entirely beyond it.
        let Some((paint_start, paint_end)) = clip_to_cursor(start, end, cursor) else {
            continue;
        };
        if paint_start > cursor {
            append_default(&mut job, &line[cursor..paint_start], font_size, default_color);
        }
        job.append(
            &line[paint_start..paint_end],
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(font_size),
                color: egui::Color32::from_rgba_unmultiplied(
                    token.color[0],
                    token.color[1],
                    token.color[2],
                    token.color[3],
                ),
                ..Default::default()
            },
        );
        cursor = paint_end;
    }

    if cursor < line.len() {
        append_default(&mut job, &line[cursor..], font_size, default_color);
    }
    job
}

/// Clips a token's `[start, end)` byte range against a monotonic `cursor`,
/// returning the not-yet-painted `[paint_start, end)` sub-range, or `None` if
/// the token lies entirely at/before `cursor` (nothing left to paint).
///
/// This is the single source of truth for the "first token wins a shared
/// column" clipping rule, shared with the line-token normalizer in
/// `service::parse` so the two layers cannot drift.
pub(crate) fn clip_to_cursor(
    start: usize,
    end: usize,
    cursor: usize,
) -> Option<(usize, usize)> {
    let paint_start = start.max(cursor);
    (end > paint_start).then_some((paint_start, end))
}

fn append_default(
    job: &mut egui::text::LayoutJob,
    text: &str,
    font_size: f32,
    color: egui::Color32,
) {
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::monospace(font_size),
            color,
            ..Default::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paint(line: &str, tokens: &[HighlightSpan]) -> String {
        let job = build_highlighted_line_job(line, tokens, 12.0, egui::Color32::GRAY);
        job.text
    }

    #[test]
    fn overlapping_tokens_never_duplicate_text() {
        // A line whose columns are covered by two overlapping tokens must paint
        // each byte exactly once. (On the pre-fix painter this input produced
        // five backticks; the six-backtick symptom is covered by
        // `trailing_newline_token_on_trimmed_line_does_not_duplicate`.)
        let line = "```";
        let color = [1, 2, 3, 255];
        let tokens = [
            HighlightSpan { start_col: 0, end_col: 3, color },
            HighlightSpan { start_col: 1, end_col: 3, color },
        ];
        assert_eq!(paint(line, &tokens), line, "overlapping tokens duplicated text");
    }

    #[test]
    fn trailing_newline_token_on_trimmed_line_does_not_duplicate() {
        // The editor trims the trailing newline before painting (display_line_text).
        // A markdown closing fence `` ``` ``
        // line carries one token covering only that newline (cols 3..4). After
        // trimming, the token clamps to zero width at the line end; the painter
        // must still emit the line exactly once and not append it again.
        let line = "```"; // trimmed: display_line_text("```\n")
        let tokens = [HighlightSpan {
            start_col: 3,
            end_col: 4, // the trimmed-away newline
            color: [1, 2, 3, 255],
        }];
        assert_eq!(paint(line, &tokens), line, "line must not be duplicated");
    }

    #[test]
    fn overlapping_tokens_cover_line_exactly_once() {
        // A later token that starts before the previous token's end and extends
        // further must still leave the line covered exactly once (no duplicates,
        // no lost columns).
        let line = "abcde";
        let color = [1, 2, 3, 255];
        let tokens = [
            HighlightSpan { start_col: 0, end_col: 2, color },
            HighlightSpan { start_col: 1, end_col: 5, color },
        ];
        assert_eq!(paint(line, &tokens), line);
    }

    #[test]
    fn disjoint_tokens_still_paint_correctly() {
        let line = "let x = 1;";
        let tokens = [
            HighlightSpan { start_col: 0, end_col: 3, color: [1, 2, 3, 255] },
            HighlightSpan { start_col: 4, end_col: 5, color: [4, 5, 6, 255] },
            HighlightSpan { start_col: 6, end_col: 11, color: [7, 8, 9, 255] },
        ];
        assert_eq!(paint(line, &tokens), line);
    }

    #[test]
    fn trailing_text_beyond_last_token_is_appended() {
        let line = "word   tail";
        let tokens = [HighlightSpan {
            start_col: 0,
            end_col: 4,
            color: [1, 2, 3, 255],
        }];
        assert_eq!(paint(line, &tokens), line);
    }
}
