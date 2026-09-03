use crate::language::HighlightSpan;
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
        if start > cursor {
            append_default(&mut job, &line[cursor..start], font_size, default_color);
        }
        if end > start {
            job.append(
                &line[start..end],
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
            cursor = end;
        }
    }

    if cursor < line.len() {
        append_default(&mut job, &line[cursor..], font_size, default_color);
    }
    job
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
