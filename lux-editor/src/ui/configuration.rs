use crate::config::{Config, EditorSettings};
use crate::events::CustomEvent;
use lux_core::Buffer;
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
            egui::ComboBox::from_id_salt("syntax_theme_select")
                .selected_text(&config_draft.theme.syntax_theme)
                .show_ui(ui, |ui| {
                    for theme in available_syntax_themes(&config_draft.theme.syntax_theme) {
                        ui.selectable_value(
                            &mut config_draft.theme.syntax_theme,
                            theme.clone(),
                            theme,
                        );
                    }
                });
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

fn available_syntax_themes(current: &str) -> Vec<String> {
    let mut themes = vec![
        "base16-ocean.dark".to_string(),
        "base16-eighties.dark".to_string(),
        "base16-mocha.dark".to_string(),
        "base16-ocean.light".to_string(),
        "InspiredGitHub".to_string(),
        "Solarized (dark)".to_string(),
        "Solarized (light)".to_string(),
    ];
    if !themes.iter().any(|theme| theme == current) {
        themes.push(current.to_string());
    }
    themes
}
