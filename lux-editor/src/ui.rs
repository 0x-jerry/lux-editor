use crate::app::ShellView;
use crate::config::{Config, EditorSettings};
use crate::events::CustomEvent;
use crate::file_tree::FileTree;
use crate::language::{HighlightSnapshot, HighlightSpan};
use lux_core::Buffer;
use std::path::PathBuf;

pub struct DrawUiState<'a> {
    pub file_tree: Option<&'a FileTree>,
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a Buffer,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    pub config_draft: &'a mut EditorSettings,
    pub config_status: Option<&'a str>,
    pub shell_view: ShellView,
}

pub fn draw_ui(ctx: &egui::Context, state: DrawUiState<'_>) -> Vec<CustomEvent> {
    let DrawUiState {
        file_tree,
        workspace_path,
        buffer,
        highlight_snapshot,
        editor_config,
        config_draft,
        config_status,
        shell_view,
    } = state;

    let mut events = Vec::new();

    draw_shell_navigation(ctx, shell_view, &mut events);

    if shell_view == ShellView::Editor
        && let Some(tree) = file_tree
    {
        egui::SidePanel::left("file_tree")
            .resizable(true)
            .default_width(200.0)
            .width_range(100.0..=500.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(event) = tree.show(ui) {
                            events.push(event);
                        }
                    });
            });
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        if shell_view == ShellView::Editor {
            render_editor_view(
                ui,
                workspace_path,
                buffer,
                highlight_snapshot,
                editor_config,
                &mut events,
            );
        } else {
            render_configuration_view(
                ui,
                workspace_path,
                buffer,
                editor_config,
                config_draft,
                config_status,
                &mut events,
            );
        }
    });

    events
}

fn draw_shell_navigation(
    ctx: &egui::Context,
    shell_view: ShellView,
    events: &mut Vec<CustomEvent>,
) {
    egui::TopBottomPanel::top("shell_navigation").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(shell_view == ShellView::Editor, "Editor")
                .clicked()
            {
                events.push(CustomEvent::SwitchToEditor);
            }

            if ui
                .selectable_label(shell_view == ShellView::Configuration, "Configuration")
                .clicked()
            {
                events.push(CustomEvent::SwitchToConfiguration);
            }
        });
    });
}

fn render_editor_view(
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

fn render_configuration_view(
    ui: &mut egui::Ui,
    workspace_path: Option<&PathBuf>,
    buffer: &Buffer,
    editor_config: &Config,
    config_draft: &mut EditorSettings,
    config_status: Option<&str>,
    events: &mut Vec<CustomEvent>,
) {
    ui.heading("Configuration");
    ui.add_space(8.0);
    ui.label(format!(
        "Workspace: {}",
        workspace_path
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "No workspace opened".to_string())
    ));
    ui.label(format!(
        "Active buffer: {}",
        buffer
            .path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "No file opened".to_string())
    ));
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(12.0);
    ui.heading("Appearance");
    ui.add_space(8.0);
    egui::Grid::new("config_editor_grid")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label("Syntax theme");
            ui.text_edit_singleline(&mut config_draft.theme.syntax_theme);
            ui.end_row();

            ui.label("Font family");
            ui.text_edit_singleline(&mut config_draft.font.family);
            ui.end_row();

            ui.label("Font size");
            ui.add(
                egui::DragValue::new(&mut config_draft.font.size)
                    .speed(0.1)
                    .range(8.0..=64.0),
            );
            ui.end_row();
        });

    let has_changes = *config_draft != editor_config.settings;
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(has_changes, egui::Button::new("Save"))
            .clicked()
        {
            events.push(CustomEvent::SaveConfiguration);
        }

        if ui
            .add_enabled(has_changes, egui::Button::new("Revert"))
            .clicked()
        {
            events.push(CustomEvent::RevertConfiguration);
        }
    });

    if let Some(status) = config_status {
        ui.add_space(8.0);
        ui.label(status);
    }
}

fn build_highlighted_line_job(
    line: &str,
    tokens: &[HighlightSpan],
    font_size: f32,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    if tokens.is_empty() {
        job.append(
            line,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(font_size),
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
            append_default(&mut job, &line[cursor..start], font_size);
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
        append_default(&mut job, &line[cursor..], font_size);
    }
    job
}

fn append_default(job: &mut egui::text::LayoutJob, text: &str, font_size: f32) {
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::monospace(font_size),
            color: egui::Color32::LIGHT_GRAY,
            ..Default::default()
        },
    );
}
