use crate::config::Config;
use crate::events::CustomEvent;
use crate::file_tree::FileTree;
use crate::language::{HighlightSnapshot, HighlightSpan};
use lux_core::Buffer;
use std::path::PathBuf;
use winit::event_loop::EventLoopProxy;

pub enum Action {
    OpenFile(PathBuf),
    OpenFolder(PathBuf),
}

pub fn draw_ui(
    ctx: &egui::Context,
    file_tree: Option<&FileTree>,
    workspace_path: Option<&PathBuf>,
    buffer: &Buffer,
    highlight_snapshot: &HighlightSnapshot,
    editor_config: &Config,
    event_proxy: &EventLoopProxy<CustomEvent>,
) -> Option<Action> {
    let mut action = None;

    if let Some(tree) = file_tree {
        egui::SidePanel::left("file_tree")
            .resizable(true)
            .default_width(200.0)
            .width_range(100.0..=500.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(path) = tree.show(ui) {
                            event_proxy.send_event(CustomEvent::OpenFile(path)).ok();
                        }
                    });
            });
    }

    egui::CentralPanel::default().show(ctx, |ui| {
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
                                action = Some(Action::OpenFile(path));
                            }
                        });
                        columns[1].vertical_centered(|ui| {
                            if ui
                                .button(egui::RichText::new("Open Folder").size(20.0))
                                .clicked()
                                && let Some(path) = rfd::FileDialog::new().pick_folder()
                            {
                                action = Some(Action::OpenFolder(path));
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
                                action = Some(Action::OpenFolder(item.path.clone()));
                            } else {
                                action = Some(Action::OpenFile(item.path.clone()));
                            }
                        }
                    }
                });
            });
        } else {
            ui.heading("Lux Editor");

            egui::ScrollArea::vertical().show(ui, |ui| {
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
                                let line_text = line.to_string();
                                if let Some(line_tokens) = highlight_snapshot.line_tokens.get(i) {
                                    let job = build_highlighted_line_job(&line_text, line_tokens);
                                    ui.label(job);
                                } else {
                                    ui.label(line_text);
                                }
                            }
                        }
                    });
            });
        }
    });

    action
}

fn build_highlighted_line_job(line: &str, tokens: &[HighlightSpan]) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    if tokens.is_empty() {
        job.append(
            line,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(14.0),
                color: egui::Color32::LIGHT_GRAY,
                ..Default::default()
            },
        );
        return job;
    }

    let mut cursor = 0usize;
    for token in tokens {
        let start = token.start_col.min(line.len());
        let end = token.end_col.min(line.len());
        if start > cursor {
            append_default(&mut job, &line[cursor..start]);
        }
        if end > start {
            job.append(
                &line[start..end],
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::monospace(14.0),
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
        append_default(&mut job, &line[cursor..]);
    }
    job
}

fn append_default(job: &mut egui::text::LayoutJob, text: &str) {
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::monospace(14.0),
            color: egui::Color32::LIGHT_GRAY,
            ..Default::default()
        },
    );
}
