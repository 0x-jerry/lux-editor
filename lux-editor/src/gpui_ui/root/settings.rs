//! Settings edits and what they re-render: the theme, the editor font, the
//! tree's exclusions, and the OS appearance the `auto` choice follows.

use super::Root;
use crate::gpui_ui::settings_view::{SettingsEvent, SettingsView};
use crate::gpui_ui::theme::apply;
use crate::settings::Config;
use crate::theme::ThemeChoice;
use gpui_kit::*;

/// Whether an OS appearance is one of the light ones.
pub(super) fn is_light(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Light | WindowAppearance::VibrantLight
    )
}

impl Root {
    /// Apply an edit from the settings tab: re-render whatever it affects and
    /// leave the write to the next tick, so typing is not one write per key.
    pub(super) fn on_settings_event(
        &mut self,
        _view: Entity<SettingsView>,
        event: &SettingsEvent,
        cx: &mut Context<Self>,
    ) {
        let SettingsEvent::Changed(settings) = event;
        let settings = (**settings).clone();
        let current = &self.state.config.settings;
        let appearance_changed = settings.theme != current.theme || settings.font != current.font;
        let excludes_changed = settings.explorer.exclude != current.explorer.exclude;

        self.state.config.settings = settings;
        self.settings_dirty = true;
        if appearance_changed {
            self.refresh_theme(cx);
        }
        if excludes_changed {
            self.apply_exclude();
        }
        cx.notify();
    }

    /// Re-publish the palette and editor font from the current settings.
    pub(super) fn refresh_theme(&self, cx: &mut Context<Self>) {
        let settings = &self.state.config.settings;
        apply(
            cx,
            ThemeChoice::from_value(&settings.theme.choice),
            &settings.font.family,
            settings.font.size,
            self.system_is_light,
        );
    }

    /// Follow a change of the OS appearance while the theme choice is `auto`.
    pub(super) fn sync_system_appearance(&mut self, window: &Window, cx: &mut Context<Self>) {
        let system_is_light = is_light(window.appearance());
        if system_is_light == self.system_is_light {
            return;
        }
        self.system_is_light = system_is_light;
        if ThemeChoice::from_value(&self.state.config.settings.theme.choice) == ThemeChoice::Auto {
            self.refresh_theme(cx);
        }
    }

    /// Write settings edited in the settings tab, at most once per tick so a
    /// burst of keystrokes is one file write.
    pub(super) fn save_settings(&mut self) {
        if !self.settings_dirty {
            return;
        }
        match Config::save_settings(&self.state.config.settings) {
            Ok(_) => self.settings_dirty = false,
            Err(err) => log::error!("failed to save settings: {err}"),
        }
    }
}
