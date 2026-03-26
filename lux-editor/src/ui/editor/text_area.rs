use super::pointer::pointer_to_line_column;
use crate::config::Config;
use crate::events::CustomEvent;
use crate::language::HighlightSnapshot;
use crate::ui::highlight::build_highlighted_line_job;
use lux_core::Buffer;

pub struct TextAreaState<'a> {
    pub buffer: &'a Buffer,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    pub caret_line: usize,
    pub caret_column: usize,
    pub selection_len: usize,
    pub caret_visible: bool,
}

pub fn render_text_area(
    ui: &mut egui::Ui,
    state: TextAreaState<'_>,
    events: &mut Vec<CustomEvent>,
) {
    let TextAreaState {
        buffer,
        highlight_snapshot,
        editor_config,
        caret_line,
        caret_column,
        selection_len,
        caret_visible,
    } = state;
    ui.heading("Lux Editor");
    ui.separator();

    let total_lines = buffer.len_lines();
    let text_style = egui::TextStyle::Monospace;
    let row_height = ui.text_style_height(&text_style);
    let font_id = text_style.resolve(ui.style());
    let char_width = ui
        .fonts_mut(|fonts| fonts.glyph_width(&font_id, 'W'))
        .max(editor_config.settings.font.size * 0.5);

    egui::ScrollArea::both()
        .scroll_source(egui::scroll_area::ScrollSource::MOUSE_WHEEL)
        .auto_shrink([false, false])
        .show_rows(ui, row_height, total_lines, |ui, row_range| {
            for i in row_range {
                if let Some(mut lines_iter) = buffer.line(i)
                    && let Some(line) = lines_iter.next()
                {
                    let line_text_owned = line.to_string();
                    let line_text_owned = line_text_owned.trim_end_matches(['\r', '\n']);
                    let line_text = if line_text_owned.is_empty() {
                        "\n"
                    } else {
                        line_text_owned
                    };

                    let response = if let Some(line_tokens) = highlight_snapshot.line_tokens.get(i)
                    {
                        let job = build_highlighted_line_job(
                            line_text,
                            line_tokens,
                            editor_config.settings.font.size,
                        );
                        ui.add(egui::Label::new(job).sense(egui::Sense::click_and_drag()))
                    } else {
                        ui.add(egui::Label::new(line_text).sense(egui::Sense::click_and_drag()))
                    };
                    if response.clicked_by(egui::PointerButton::Primary)
                        && let Some(pointer) = response.interact_pointer_pos()
                    {
                        let selecting = ui.input(|input| input.modifiers.shift);
                        let (_, column) = pointer_to_line_column(
                            pointer,
                            response.rect,
                            row_height,
                            i,
                            char_width,
                        );
                        events.push(CustomEvent::SetCaretFromPointer {
                            line_index: i,
                            column,
                            selecting,
                        });
                    }
                    if response.dragged_by(egui::PointerButton::Primary)
                        && let Some(pointer) = response.interact_pointer_pos()
                    {
                        let (line_index, column) = pointer_to_line_column(
                            pointer,
                            response.rect,
                            row_height,
                            i,
                            char_width,
                        );
                        events.push(CustomEvent::SetCaretFromPointer {
                            line_index,
                            column,
                            selecting: true,
                        });
                    }
                    if selection_len == 0 && caret_visible && i + 1 == caret_line {
                        let x = response.rect.min.x
                            + (caret_column.saturating_sub(1) as f32 * char_width);
                        ui.painter().line_segment(
                            [
                                egui::pos2(x, response.rect.top()),
                                egui::pos2(x, response.rect.bottom()),
                            ],
                            egui::Stroke::new(1.5, ui.visuals().text_color()),
                        );
                    }
                }
            }
        });
}
