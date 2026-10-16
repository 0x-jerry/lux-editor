//! The settings tab: the editable surface over the persisted configuration.
//!
//! One page per settings section, one row per field, generated with the kit's
//! `setting` component (which brings the page sidebar and the search box). Every
//! edit is published as a whole [`EditorSettings`] value; the root applies it,
//! debounces the write to disk and re-renders from it, so this view owns no
//! settings state that outlives the tab.

use crate::settings::EditorSettings;
use gpui_kit::component::setting::{
    NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_kit::*;

/// A settings edit: the complete new value of the configuration.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingsEvent {
    Changed(Box<EditorSettings>),
}

/// Dropdown entries as the kit takes them: `(value, label)`.
const THEME_CHOICES: [(&str, &str); 3] = [("auto", "Auto"), ("dark", "Dark"), ("light", "Light")];
const LOG_LEVELS: [(&str, &str); 5] = [
    ("error", "Error"),
    ("warn", "Warn"),
    ("info", "Info"),
    ("debug", "Debug"),
    ("trace", "Trace"),
];

const FONT_SIZE: NumberFieldOptions = NumberFieldOptions {
    min: 8.0,
    max: 64.0,
    step: 1.0,
};

pub struct SettingsView {
    draft: EditorSettings,
    /// The exclude patterns exactly as typed. The parsed list cannot be rendered
    /// back into the input without rewriting it mid-edit.
    exclude_text: String,
    /// Built once: the kit boxes each field behind an `Rc`, so rebuilding the
    /// tree per frame would re-create the fields.
    pages: Vec<SettingPage>,
}

impl SettingsView {
    pub fn new(settings: &EditorSettings, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let draft = settings.clone();
        let exclude_text = join_patterns(&draft.explorer.exclude);
        let pages = pages(&cx.weak_entity());
        Self {
            draft,
            exclude_text,
            pages,
        }
    }

    /// Adopt settings changed outside the view (the config file watcher).
    pub fn sync(&mut self, settings: &EditorSettings, cx: &mut Context<Self>) {
        if self.draft == *settings {
            return;
        }
        self.draft = settings.clone();
        self.exclude_text = join_patterns(&settings.explorer.exclude);
        cx.notify();
    }

    /// Publish the draft: the root persists it and re-renders whatever it
    /// affects.
    fn publish(&self, cx: &mut Context<Self>) {
        cx.emit(SettingsEvent::Changed(Box::new(self.draft.clone())));
        cx.notify();
    }
}

impl EventEmitter<SettingsEvent> for SettingsView {}

/// The exclude list as the single text input shows it.
fn join_patterns(patterns: &[String]) -> String {
    patterns.join(", ")
}

/// Split that text back into patterns; blank entries are dropped.
fn split_patterns(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|pattern| !pattern.is_empty())
        .map(str::to_string)
        .collect()
}

fn choices(pairs: &[(&str, &str)]) -> Vec<(SharedString, SharedString)> {
    pairs
        .iter()
        .map(|(value, label)| (SharedString::from(*value), SharedString::from(*label)))
        .collect()
}

/// Read a setting through the view. The getter runs from this view's own render
/// tree, so the entity is alive; a torn-down view reads as its default.
fn getter<T: Clone + Default + 'static>(
    view: &WeakEntity<SettingsView>,
    read: impl Fn(&SettingsView) -> T + 'static,
) -> impl Fn(&App) -> T + 'static {
    let view = view.clone();
    move |cx: &App| match view.upgrade() {
        Some(view) => read(&view.read(cx)),
        None => T::default(),
    }
}

/// Store a setting and publish the whole configuration.
fn setter<T: 'static>(
    view: &WeakEntity<SettingsView>,
    write: impl Fn(&mut SettingsView, T) + 'static,
) -> impl Fn(T, &mut App) + 'static {
    let view = view.clone();
    move |value: T, cx: &mut App| {
        let _ = view.update(cx, |view, cx| {
            write(view, value);
            view.publish(cx);
        });
    }
}

fn pages(view: &WeakEntity<SettingsView>) -> Vec<SettingPage> {
    vec![
        SettingPage::new("Appearance")
            .description("Colors, theme and the code font.")
            .group(SettingGroup::new().items([
                SettingItem::new(
                    "Theme",
                    SettingField::dropdown(
                        choices(&THEME_CHOICES),
                        getter(view, |view| {
                            SharedString::from(view.draft.theme.choice.as_str())
                        }),
                        setter(view, |view, value: SharedString| {
                            view.draft.theme.choice = value.to_string()
                        }),
                    )
                    .default_value("auto"),
                )
                .description("App theme; Auto follows the operating system."),
                SettingItem::new(
                    "Font family",
                    SettingField::input(
                        getter(view, |view| {
                            SharedString::from(view.draft.font.family.as_str())
                        }),
                        setter(view, |view, value: SharedString| {
                            view.draft.font.family = value.to_string()
                        }),
                    )
                    .default_value(""),
                )
                .description("Monospace font family used for code; empty uses the system default."),
                SettingItem::new(
                    "Font size",
                    SettingField::number_input(
                        FONT_SIZE,
                        getter(view, |view| f64::from(view.draft.font.size)),
                        setter(view, |view, value: f64| view.draft.font.size = value as f32),
                    )
                    .default_value(14.0),
                )
                .description("Base font size in points (8–64)."),
            ])),
        SettingPage::new("Formatting")
            .description("How documents are piped to an external formatter.")
            .group(SettingGroup::new().items([
                SettingItem::new(
                    "Formatter command",
                    SettingField::input(
                        getter(view, |view| {
                            SharedString::from(view.draft.formatter.command.as_str())
                        }),
                        setter(view, |view, value: SharedString| {
                            view.draft.formatter.command = value.to_string()
                        }),
                    )
                    .default_value(""),
                )
                .description(
                    "The document is piped to the command on stdin and replaced with its \
                     stdout. An empty command disables formatting.",
                ),
                SettingItem::new(
                    "Arguments",
                    SettingField::input(
                        getter(view, |view| {
                            SharedString::from(view.draft.formatter.args.as_str())
                        }),
                        setter(view, |view, value: SharedString| {
                            view.draft.formatter.args = value.to_string()
                        }),
                    )
                    .default_value("--stdin"),
                )
                .description("Arguments are split on whitespace."),
                SettingItem::new(
                    "Format on save",
                    SettingField::switch(
                        getter(view, |view| view.draft.formatter.format_on_save),
                        setter(view, |view, value: bool| {
                            view.draft.formatter.format_on_save = value
                        }),
                    )
                    .default_value(true),
                )
                .description("Run the formatter before writing the file."),
            ])),
        SettingPage::new("Explorer")
            .description("What the file tree shows.")
            .group(SettingGroup::new().items([
                SettingItem::new(
                    "Exclude patterns",
                    SettingField::input(
                        getter(view, |view| SharedString::from(view.exclude_text.as_str())),
                        setter(view, |view, value: SharedString| {
                            let text = value.to_string();
                            view.draft.explorer.exclude = split_patterns(&text);
                            view.exclude_text = text;
                        }),
                    )
                    .default_value(".git"),
                )
                .description(
                    "Gitignore-style patterns left out of the file tree, comma separated; a \
                     bare name matches at any depth (`/.git` only the root). Gitignored \
                     entries are dimmed rather than excluded.",
                ),
            ])),
        SettingPage::new("Logging")
            .description("Diagnostics written to the terminal.")
            .group(SettingGroup::new().items([SettingItem::new(
                "Log level",
                SettingField::dropdown(
                    choices(&LOG_LEVELS),
                    getter(view, |view| {
                        SharedString::from(view.draft.log.level.as_str())
                    }),
                    setter(view, |view, value: SharedString| {
                        view.draft.log.level = value.to_string()
                    }),
                )
                .default_value("warn"),
            )
            .description("Console verbosity; takes effect on restart. RUST_LOG overrides it.")])),
    ]
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            RenderOnce::render(
                Settings::new("lux-settings").pages(self.pages.clone()),
                window,
                cx,
            )
            .into_any_element(),
        )
    }
}
