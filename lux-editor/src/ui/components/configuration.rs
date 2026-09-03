use crate::config::{Config, EditorSettings};
use crate::events::ConfigurationEvent;
use crate::ui::component::Component;
use crate::ui::theme::{ThemeChoice, syntax_theme_for};
use eframe::egui;
use lux_core::Buffer;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Settings editor for the configuration tab. Owns the editable draft, the
/// save status and the debounced autosave; emits the saved settings so the
/// app can apply theme/font side effects.
#[derive(Default)]
pub struct ConfigurationView {
    draft: EditorSettings,
    status: Option<String>,
    autosave_deadline: Option<Instant>,
}

impl ConfigurationView {
    const AUTOSAVE_DELAY: Duration = Duration::from_millis(350);

    pub fn sync_draft(&mut self, settings: &EditorSettings) {
        self.draft = settings.clone();
        self.status = None;
        self.autosave_deadline = None;
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    fn schedule_autosave(&mut self) {
        self.autosave_deadline = Some(Instant::now() + Self::AUTOSAVE_DELAY);
        self.status = None;
    }

    /// Persists the draft once the debounce elapsed; returns the saved
    /// settings when they differ from what the app already has.
    fn flush_autosave(&mut self, config: &Config) -> Option<EditorSettings> {
        let deadline = self.autosave_deadline?;
        if Instant::now() < deadline {
            return None;
        }
        self.autosave_deadline = None;
        if self.draft == config.settings {
            return None;
        }
        if Config::save_settings(&self.draft).is_ok() {
            self.status = Some("Configuration saved".to_string());
            Some(self.draft.clone())
        } else {
            self.status = Some("Failed to save configuration".to_string());
            None
        }
    }
}

pub struct ConfigurationViewInput<'a> {
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a Buffer,
    pub editor_config: &'a Config,
}

impl Component for ConfigurationView {
    type Message = ConfigurationEvent;
    type Input<'a> = ConfigurationViewInput<'a>;

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        input: Self::Input<'_>,
    ) -> Vec<ConfigurationEvent> {
        let ConfigurationViewInput {
            workspace_path,
            buffer,
            editor_config,
        } = input;
        let mut events = Vec::new();
        if let Some(saved) = self.flush_autosave(editor_config) {
            events.push(ConfigurationEvent::ConfigurationSaved(saved));
        }
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
                    let current_choice = ThemeChoice::from_value(&self.draft.theme.choice);
                    egui::ComboBox::from_id_salt("theme_choice_select")
                        .selected_text(current_choice.label())
                        .show_ui(ui, |ui| {
                            for choice in [ThemeChoice::Auto, ThemeChoice::Dark, ThemeChoice::Light]
                            {
                                if ui
                                    .selectable_value(
                                        &mut self.draft.theme.choice,
                                        choice.value().to_string(),
                                        choice.label(),
                                    )
                                    .changed()
                                {
                                    // Coupled: the syntax theme follows the chrome
                                    // theme so token/background contrast holds.
                                    self.draft.theme.syntax_theme =
                                        syntax_theme_for(choice, ui.ctx().system_theme())
                                            .to_string();
                                    self.draft.theme.theme_path = None;
                                    changed = true;
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Syntax theme");
                    ui.label(syntax_label(&self.draft, ui.ctx().system_theme()));
                    ui.end_row();

                    ui.label("Font family");
                    if ui
                        .text_edit_singleline(&mut self.draft.font.family)
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Font size");
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.draft.font.size)
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
                        .text_edit_singleline(&mut self.draft.formatter.command)
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Arguments");
                    if ui
                        .text_edit_singleline(&mut self.draft.formatter.args)
                        .changed()
                    {
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Format on save");
                    if ui
                        .checkbox(&mut self.draft.formatter.format_on_save, "")
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
                        .checkbox(&mut self.draft.behavior.smart_pairing, "")
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

        if changed {
            self.schedule_autosave();
        }

        if let Some(status) = self.status.as_deref() {
            ui.add_space(8.0);
            ui.label(status);
        }

        events
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
