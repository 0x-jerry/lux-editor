use super::highlight::build_highlighted_line_job;
use crate::config::Config;
use crate::events::CustomEvent;
use crate::language::HighlightSnapshot;
use lux_core::Buffer;
use std::path::PathBuf;

pub struct EditorViewState<'a> {
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a Buffer,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    pub caret_line: usize,
    pub caret_column: usize,
    pub selection_len: usize,
    pub caret_visible: bool,
}

pub fn render_editor_view(
    ui: &mut egui::Ui,
    state: EditorViewState<'_>,
    events: &mut Vec<CustomEvent>,
) {
    let EditorViewState {
        workspace_path,
        buffer,
        highlight_snapshot,
        editor_config,
        caret_line,
        caret_column,
        selection_len,
        caret_visible,
    } = state;
    if workspace_path.is_none() && buffer.path().is_none() {
        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.heading(egui::RichText::new("Lux Editor").size(48.0).strong());
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.columns(2, |columns| {
                    columns[0].vertical_centered(|ui| {
                        if ui
                            .button(egui::RichText::new("Open File").size(20.0))
                            .clicked()
                            && let Some(path) = rfd::FileDialog::new().pick_file()
                        {
                            events.push(CustomEvent::OpenFile(path));
                        }
                    });
                    columns[1].vertical_centered(|ui| {
                        if ui
                            .button(egui::RichText::new("Open Folder").size(20.0))
                            .clicked()
                            && let Some(path) = rfd::FileDialog::new().pick_folder()
                        {
                            events.push(CustomEvent::OpenFolder(path));
                        }
                    });
                });
            });

            ui.add_space(40.0);
            ui.separator();
            ui.add_space(20.0);
            ui.heading("Recent Items");
            ui.add_space(10.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                for item in &editor_config.recent_items {
                    let label = format!(
                        "{} ({})",
                        item.path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown"),
                        item.path.display()
                    );
                    if ui.selectable_label(false, label).clicked() {
                        if item.is_dir {
                            events.push(CustomEvent::OpenFolder(item.path.clone()));
                        } else {
                            events.push(CustomEvent::OpenFile(item.path.clone()));
                        }
                    }
                }
            });
        });
        return;
    }

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
        .auto_shrink([false; 2])
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

fn pointer_to_line_column(
    pointer: egui::Pos2,
    line_rect: egui::Rect,
    row_height: f32,
    row_index: usize,
    char_width: f32,
) -> (usize, usize) {
    let row_delta = ((pointer.y - line_rect.top()) / row_height).floor() as isize;
    let line_index = (row_index as isize + row_delta).max(0) as usize;
    let column = ((pointer.x - line_rect.left()) / char_width)
        .floor()
        .max(0.0) as usize;
    (line_index, column)
}
