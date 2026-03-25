use super::highlight::build_highlighted_line_job;
use crate::config::Config;
use crate::events::CustomEvent;
use crate::language::HighlightSnapshot;
use lux_core::Buffer;
use std::path::PathBuf;

pub fn render_editor_view(
    ui: &mut egui::Ui,
    workspace_path: Option<&PathBuf>,
    buffer: &Buffer,
    highlight_snapshot: &HighlightSnapshot,
    editor_config: &Config,
    events: &mut Vec<CustomEvent>,
) {
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

    let total_lines = buffer.len_lines();
    let text_style = egui::TextStyle::Monospace;
    let row_height = ui.text_style_height(&text_style);
    ui.spacing_mut().item_spacing.y = 0.0;

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show_rows(ui, row_height, total_lines, |ui, row_range| {
            for i in row_range {
                if let Some(mut lines_iter) = buffer.line(i)
                    && let Some(line) = lines_iter.next()
                {
                    let line_text_owned = line.to_string();
                    let line_text = line_text_owned.trim_end_matches(['\r', '\n']);
                    if let Some(line_tokens) = highlight_snapshot.line_tokens.get(i) {
                        let job = build_highlighted_line_job(
                            line_text,
                            line_tokens,
                            editor_config.settings.font.size,
                        );
                        ui.label(job);
                    } else {
                        ui.label(line_text);
                    }
                }
            }
        });
}
