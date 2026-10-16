//! The settings tab: an editable view over the persisted configuration.
//!
//! Edits are published as whole [`EditorSettings`] values; the root applies,
//! saves and (for theme and font) re-renders from them, so this view owns no
//! state that outlives the tab.

use super::theme::Palette;
use crate::settings::EditorSettings;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// A settings edit: the complete new value of the configuration's editor
/// section.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingsEvent {
    Changed(Box<EditorSettings>),
}

/// The choices of `EditorSettings::theme::choice`, in display order.
const THEMES: [&str; 3] = ["auto", "dark", "light"];

pub struct SettingsView {
    draft: EditorSettings,
}

impl SettingsView {
    pub fn new(settings: &EditorSettings, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _ = cx;
        Self {
            draft: settings.clone(),
        }
    }

    /// Adopt settings changed outside the view (the config file watcher).
    pub fn sync(&mut self, settings: &EditorSettings, cx: &mut Context<Self>) {
        if self.draft == *settings {
            return;
        }
        self.draft = settings.clone();
        cx.notify();
    }

    fn set(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut EditorSettings)) {
        edit(&mut self.draft);
        cx.emit(SettingsEvent::Changed(Box::new(self.draft.clone())));
        cx.notify();
    }

    fn row(label: &'static str, value: String, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::get(cx);
        h_flex()
            .items_center()
            .gap_4()
            .py_1()
            .child(div().w(px(200.0)).flex_shrink_0().child(label))
            .child(div().text_color(palette.weak).child(value))
    }

    fn theme_choices(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::get(cx);
        let current = self.draft.theme.choice.clone();
        h_flex().gap_2().children(THEMES.map(|choice| {
            let selected = choice == current;
            div()
                .id(SharedString::from(format!("theme-{choice}")))
                .px_2()
                .py_1()
                .rounded_sm()
                .cursor_pointer()
                .border_1()
                .border_color(palette.border)
                .when(selected, |chip| chip.bg(palette.active))
                .when(!selected, |chip| chip.hover(|style| style.bg(palette.hover)))
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.set(cx, |settings| settings.theme.choice = choice.to_string())
                }))
                .child(choice)
        }))
    }
}

impl EventEmitter<SettingsEvent> for SettingsView {}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::get(cx);
        let font = format!(
            "{} — {} pt",
            if self.draft.font.family.is_empty() {
                "system default"
            } else {
                &self.draft.font.family
            },
            self.draft.font.size
        );
        let formatter = if self.draft.formatter.command.is_empty() {
            "off".to_string()
        } else {
            format!(
                "{} {}",
                self.draft.formatter.command, self.draft.formatter.args
            )
        };
        let exclude = self.draft.explorer.exclude.join(", ");

        v_flex()
            .id("settings")
            .size_full()
            .overflow_y_scroll()
            .p_6()
            .gap_1()
            .text_sm()
            .child(div().text_lg().pb_2().child("Settings"))
            .child(
                h_flex()
                    .items_center()
                    .gap_4()
                    .py_1()
                    .child(div().w(px(200.0)).flex_shrink_0().child("Theme"))
                    .child(self.theme_choices(cx)),
            )
            .child(Self::row("Font", font, cx))
            .child(Self::row("Formatter", formatter, cx))
            .child(Self::row(
                "Format on save",
                self.draft.formatter.format_on_save.to_string(),
                cx,
            ))
            .child(Self::row("Explorer exclude", exclude, cx))
            .child(Self::row("Log level", self.draft.log.level.clone(), cx))
            .child(
                div()
                    .pt_4()
                    .text_color(palette.weak)
                    .child("Settings are saved to the user config file as they change."),
            )
    }
}
