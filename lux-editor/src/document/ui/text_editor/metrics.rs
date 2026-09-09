use crate::settings::Config;
use eframe::egui;

pub struct TextEditorMetrics {
    pub row_height: f32,
    pub char_width: f32,
    pub gutter_total_width: f32,
}

pub fn measure_text_editor(
    ui: &mut egui::Ui,
    total_lines: usize,
    editor_config: &Config,
) -> TextEditorMetrics {
    let text_style = egui::TextStyle::Monospace;
    let row_height = ui.text_style_height(&text_style);
    let font_id = text_style.resolve(ui.style());
    let char_width = ui
        .fonts_mut(|fonts| fonts.glyph_width(&font_id, 'W'))
        .max(editor_config.settings.font.size * 0.5);
    let gutter_digits = total_lines.max(1).to_string().len().max(4);
    let gutter_width = (gutter_digits as f32 * char_width) + (char_width * 2.0);

    TextEditorMetrics {
        row_height,
        char_width,
        gutter_total_width: gutter_width + 5.0,
    }
}
