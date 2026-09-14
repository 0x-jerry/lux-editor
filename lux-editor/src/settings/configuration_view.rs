use crate::chrome::ui::widgets::icon_button;
use crate::component::Component;
use crate::events::ConfigurationEvent;
use crate::settings::schema::{BUILTIN, RowSchema, RowType, SectionSchema};
use crate::settings::{Config, EditorSettings};
use eframe::egui;
use egui_phosphor::regular::{MAGNIFYING_GLASS, PLUS, X};
use serde_json::Value;
use std::time::{Duration, Instant};

/// Settings editor for the configuration tab. Owns the editable draft (a JSON
/// value mirrored from the typed settings), the save status and the debounced
/// autosave; emits the saved settings so the app can apply theme/font side
/// effects. The UI is generated from [`BUILTIN`]: every row's `RowType` picks
/// its widget and its JSON pointer binds the value.
#[derive(Default)]
pub struct ConfigurationView {
    draft: Value,
    status: Option<String>,
    autosave_deadline: Option<Instant>,
    search: String,
}

impl ConfigurationView {
    const AUTOSAVE_DELAY: Duration = Duration::from_millis(350);

    pub fn sync_draft(&mut self, settings: &EditorSettings) {
        self.draft = serde_json::to_value(settings).expect("settings serialize to json");
        self.status = None;
        self.autosave_deadline = None;
        self.search.clear();
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
        let Ok(draft) = serde_json::from_value::<EditorSettings>(self.draft.clone()) else {
            return None;
        };
        if draft == config.settings {
            return None;
        }
        if Config::save_settings(&draft).is_ok() {
            self.status = Some("Saved".to_string());
            Some(draft)
        } else {
            self.status = Some("Failed to save".to_string());
            None
        }
    }
}

pub struct ConfigurationViewInput<'a> {
    pub editor_config: &'a Config,
}

/// Width of the settings column; rows are left-aligned inside it.
const CONTENT_WIDTH: f32 = 720.0;
/// Cap for text inputs so they don't span the whole column.
const FIELD_WIDTH: f32 = 420.0;

impl Component for ConfigurationView {
    type Message = ConfigurationEvent;
    type Input<'a> = ConfigurationViewInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<ConfigurationEvent> {
        let ConfigurationViewInput { editor_config } = input;
        let mut events = Vec::new();
        if let Some(saved) = self.flush_autosave(editor_config) {
            events.push(ConfigurationEvent::ConfigurationSaved(saved));
        }

        let query = self.search.trim().to_lowercase();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(CONTENT_WIDTH);
                        let mut changed = false;
                        let mut any_visible = false;

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Configuration").size(24.0).strong());
                            if let Some(status) = self.status.as_deref() {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let color = if status.starts_with("Failed") {
                                            ui.visuals().error_fg_color
                                        } else {
                                            ui.visuals().weak_text_color()
                                        };
                                        ui.label(egui::RichText::new(status).small().color(color));
                                    },
                                );
                            }
                        });
                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(MAGNIFYING_GLASS)
                                    .color(ui.visuals().weak_text_color()),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.search)
                                    .id_salt("config_search")
                                    .hint_text("Search settings")
                                    .desired_width(f32::INFINITY),
                            );
                        });
                        ui.add_space(20.0);

                        for section in BUILTIN.sections {
                            let rows = section.visible_rows(&query);
                            if rows.is_empty() {
                                continue;
                            }
                            any_visible = true;
                            section_header(ui, section);
                            for row in rows {
                                changed |= setting_row(ui, row.title, row.description, |ui| {
                                    control(ui, &mut self.draft, row)
                                });
                            }
                        }

                        if !any_visible {
                            ui.label(
                                egui::RichText::new("No matching settings")
                                    .italics()
                                    .color(ui.visuals().weak_text_color()),
                            );
                        }

                        if changed {
                            self.schedule_autosave();
                        }
                        ui.add_space(24.0);
                    });
                    ui.add_space(24.0);
                });
            });

        events
    }
}

/// One control per row, generated from the row's type. Reads/writes the draft
/// value through the row's dot path; returns whether the control changed.
fn control(ui: &mut egui::Ui, draft: &mut Value, row: &RowSchema) -> bool {
    let Some(node) = dot_path_mut(draft, row.path) else {
        return false;
    };
    match row.kind {
        RowType::Text { .. } => match node {
            Value::String(s) => ui
                .add(
                    egui::TextEdit::singleline(s)
                        .id_salt(row.path)
                        .desired_width(FIELD_WIDTH),
                )
                .changed(),
            _ => false,
        },
        RowType::Number { min, max, .. } => {
            let Some(value) = node.as_f64() else {
                return false;
            };
            let mut current = value;
            let changed = ui
                .add(
                    egui::DragValue::new(&mut current)
                        .speed(0.1)
                        .range(min..=max),
                )
                .changed();
            if changed {
                *node = serde_json::json!(current);
            }
            changed
        }
        RowType::Bool { label, .. } => match node {
            Value::Bool(b) => ui.checkbox(b, label).changed(),
            _ => false,
        },
        RowType::TextList { .. } => match node {
            Value::Array(items) => text_list(ui, row.path, items),
            _ => false,
        },
        RowType::TextChoice { options, .. } => {
            let Value::String(current) = &*node else {
                return false;
            };
            let current = current.clone();
            let mut row_changed = false;
            let selected = options
                .iter()
                .find(|(_, value)| *value == current)
                .map(|(label, _)| *label)
                .unwrap_or(options[0].0);
            egui::ComboBox::from_id_salt(row.path)
                .width(200.0)
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for &(label, value) in options {
                        if ui
                            .selectable_value(node, serde_json::json!(value), label)
                            .changed()
                        {
                            row_changed = true;
                        }
                    }
                });
            row_changed
        }
    }
}

/// One editable row per string item plus an add button. Blank rows stay in the
/// draft (pruning them would fight the caret); consumers ignore them.
fn text_list(ui: &mut egui::Ui, path: &str, items: &mut Vec<Value>) -> bool {
    let mut changed = false;
    let mut remove = None;
    for (index, item) in items.iter_mut().enumerate() {
        let Value::String(text) = item else {
            continue;
        };
        ui.horizontal(|ui| {
            changed |= ui
                .add(
                    egui::TextEdit::singleline(text)
                        .id_salt((path, index))
                        .desired_width(FIELD_WIDTH),
                )
                .changed();
            if icon_button(ui, X, false).clicked() {
                remove = Some(index);
            }
        });
    }
    if let Some(index) = remove {
        items.remove(index);
        changed = true;
    }
    if ui.button(format!("{PLUS} Add")).clicked() {
        items.push(Value::String(String::new()));
        changed = true;
    }
    changed
}

/// Resolve a dot-separated key (e.g. "font.size") to a mutable value node.
fn dot_path_mut<'a>(root: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let mut node = root;
    for segment in path.split('.') {
        node = node.as_object_mut()?.get_mut(segment)?;
    }
    Some(node)
}

/// A flat section title with a muted description and a divider underneath.
fn section_header(ui: &mut egui::Ui, section: &SectionSchema) {
    ui.label(egui::RichText::new(section.title).strong());
    if !section.description.is_empty() {
        ui.label(
            egui::RichText::new(section.description)
                .font(description_font(ui))
                .color(description_color(ui.visuals())),
        );
    }
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(12.0);
}

/// One setting: bold title, muted description, control below. Returns whether
/// the control changed this frame.
fn setting_row(
    ui: &mut egui::Ui,
    title: &str,
    description: &str,
    control: impl FnOnce(&mut egui::Ui) -> bool,
) -> bool {
    ui.label(egui::RichText::new(title).strong());
    if !description.is_empty() {
        ui.label(
            egui::RichText::new(description)
                .font(description_font(ui))
                .color(description_color(ui.visuals())),
        );
    }
    ui.add_space(6.0);
    let changed = control(ui);
    ui.add_space(18.0);
    changed
}

/// Helper type derived from the body size so descriptions scale with the
/// configured font; `TextStyle::Small` stays at egui's fixed 9px default.
fn description_font(ui: &egui::Ui) -> egui::FontId {
    let body = egui::TextStyle::Body.resolve(ui.style());
    egui::FontId::new(description_size(body.size), body.family)
}

fn description_size(body_size: f32) -> f32 {
    body_size.max(11.0)
}

/// Muted helper color blended from the body text toward the panel background:
/// subordinate to the bold title, but above WCAG AA on both themes.
fn description_color(visuals: &egui::Visuals) -> egui::Color32 {
    visuals
        .text_color()
        .lerp_to_gamma(visuals.panel_fill, DESCRIPTION_BLEND)
}

/// Fraction of the panel background mixed into description text.
const DESCRIPTION_BLEND: f32 = 0.12;

#[cfg(test)]
mod tests {
    use super::{description_color, description_size};
    use crate::theme::{AppTheme, ThemeChoice};
    use eframe::egui;

    #[test]
    fn description_size_tracks_the_body_with_a_legibility_floor() {
        assert_eq!(description_size(24.0), 24.0);
        assert_eq!(description_size(14.0), 14.0);
        assert_eq!(description_size(9.0), 11.0);
    }

    fn luminance(color: egui::Color32) -> f32 {
        let channel = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.03928 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    fn contrast(a: egui::Color32, b: egui::Color32) -> f32 {
        let (hi, lo) = (luminance(a).max(luminance(b)), luminance(a).min(luminance(b)));
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn descriptions_keep_readable_contrast_on_both_themes() {
        for choice in [ThemeChoice::Dark, ThemeChoice::Light] {
            let visuals = AppTheme::resolve(choice).visuals;
            let ratio = contrast(description_color(&visuals), visuals.panel_fill);
            assert!(
                ratio >= 4.5,
                "{choice:?} description contrast {ratio:.2} is below WCAG AA"
            );
        }
    }
}
