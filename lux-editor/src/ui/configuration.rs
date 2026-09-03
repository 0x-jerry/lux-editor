use crate::config::{Config, EditorSettings};
use crate::events::CustomEvent;
use crate::ui::theme::{ThemeChoice, syntax_theme_for};
use lux_core::Buffer;
use eframe::egui;
use std::path::PathBuf;

pub fn render_configuration_view(
    ui: &mut egui::Ui,
    workspace_path: Option<&PathBuf>,
    buffer: &Buffer,
    editor_config: &Config,
    config_draft: &mut EditorSettings,
    config_status: Option<&str>,
    events: &mut Vec<CustomEvent>,
) {
    let mut changed = false;

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
    ui.label(format!(
        "Settings file: {}",
        Config::user_settings_path().display()
    ));

    section(ui, "Appearance", |ui| {
        egui::Grid::new("config_editor_grid")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label("Theme");
                let current_choice = ThemeChoice::from_value(&config_draft.theme.choice);
                egui::ComboBox::from_id_salt("theme_choice_select")
                    .selected_text(current_choice.label())
                    .show_ui(ui, |ui| {
                        for choice in [
                            ThemeChoice::Auto,
                            ThemeChoice::Dark,
                            ThemeChoice::Light,
                        ] {
                            if ui
                                .selectable_value(
                                    &mut config_draft.theme.choice,
                                    choice.value().to_string(),
                                    choice.label(),
                                )
                                .changed()
                            {
                                // Coupled: the syntax theme follows the chrome
                                // theme so token/background contrast holds.
                                config_draft.theme.syntax_theme =
                                    syntax_theme_for(choice, ui.ctx().system_theme()).to_string();
                                config_draft.theme.theme_path = None;
                                changed = true;
                            }
                        }
                    });
                ui.end_row();

                ui.label("Syntax theme");
                ui.label(syntax_label(config_draft, ui.ctx().system_theme()));
                ui.end_row();

                ui.label("Font family");
                if ui
                    .text_edit_singleline(&mut config_draft.font.family)
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Font size");
                if ui
                    .add(
                        egui::DragValue::new(&mut config_draft.font.size)
                            .speed(0.1)
                            .range(8.0..=64.0),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
            });
    });

    section(ui, "Formatting", |ui| {
        egui::Grid::new("config_formatter_grid")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label("Formatter command");
                if ui
                    .text_edit_singleline(&mut config_draft.formatter.command)
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Arguments");
                if ui
                    .text_edit_singleline(&mut config_draft.formatter.args)
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();

                ui.label("Format on save");
                if ui
                    .checkbox(&mut config_draft.formatter.format_on_save, "")
                    .on_hover_text("Run the formatter before writing the file")
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
            });
        ui.label(
            "The document is piped to the command on stdin and replaced with \
             its stdout. Arguments are split on whitespace. An empty command \
             disables formatting.",
        );
    });

    section(ui, "Typing", |ui| {
        egui::Grid::new("config_typing_grid")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label("Smart bracket pairing");
                if ui
                    .checkbox(&mut config_draft.behavior.smart_pairing, "")
                    .on_hover_text(
                        "Auto-close brackets and quotes, skip over closing partners, \
                         delete empty pairs with Backspace, and open blank lines \
                         inside empty pairs with Enter",
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.end_row();
            });
    });

    if changed && *config_draft != editor_config.settings {
        events.push(CustomEvent::ConfigurationDraftChanged);
    }

    if let Some(status) = config_status {
        ui.add_space(8.0);
        ui.label(status);
    }
}

/// A titled card. Keeps the page scannable as sections grow.
fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.heading(title);
            ui.add_space(8.0);
            body(ui);
        });
    ui.add_space(10.0);
}

fn syntax_label(settings: &EditorSettings, system: Option<egui::Theme>) -> String {
    if let Some(path) = &settings.theme.theme_path {
        format!("Custom: {}", path.display())
    } else {
        let choice = ThemeChoice::from_value(&settings.theme.choice);
        format!("Syntax: {}", syntax_theme_for(choice, system))
    }
}