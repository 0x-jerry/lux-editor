//! Settings actions: config-file refresh, the configuration view's autosave,
//! recent-file bookkeeping and the style-refresh flag that config changes set.

use crate::app::Ctx;
use crate::events::{AppEvent, ConfigurationEvent};
use crate::settings::{Config, EditorSettings, SettingsState};
use std::time::{Duration, Instant};

impl Ctx<'_> {
    /// App-level state & navigation: config refresh and open commands.
    pub(crate) fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::ConfigChange => self.on_config_change(),
            AppEvent::OpenFile(path) => self.open_file(path),
            AppEvent::OpenFolder(path) => self.open_folder(path),
            AppEvent::ClearRecentItems => self.settings.editor_config.clear_recent_items(),
        }
    }

    /// Configuration: the configuration view autosave.
    pub(crate) fn handle_configuration_event(&mut self, event: ConfigurationEvent) {
        match event {
            ConfigurationEvent::ConfigurationSaved(settings) => {
                self.apply_saved_configuration(settings)
            }
        }
    }

    pub(crate) fn restart_settings_watcher(&mut self) {
        let watch_roots = Config::settings_watch_roots();
        self.settings.watcher = SettingsState::start_watcher(
            &watch_roots,
            self.runtime.event_tx.clone(),
            self.egui_ctx().clone(),
        );
    }

    pub(crate) fn apply_saved_configuration(&mut self, settings: EditorSettings) {
        let theme_changed = settings.theme != self.settings.editor_config.settings.theme;
        let font_changed = settings.font != self.settings.editor_config.settings.font;
        self.settings.editor_config.settings = settings;
        self.chrome.needs_style_refresh |= theme_changed || font_changed;
    }

    pub(crate) fn on_config_change(&mut self) {
        if self.settings.editor_config.reload_settings() {
            self.chrome.needs_style_refresh = true;
        }
        self.chrome
            .shell
            .sync_config_draft(&self.settings.editor_config.settings);
    }

    /// Debounced recent-config flush: changes land at most one save per
    /// 500 ms window instead of a synchronous disk write per mutation.
    pub(crate) fn flush_recent_config(&mut self) {
        if !self.settings.editor_config.recent_dirty {
            self.frame.recent_flush_deadline = None;
            return;
        }
        let now = Instant::now();
        let deadline = *self
            .frame
            .recent_flush_deadline
            .get_or_insert_with(|| now + Duration::from_millis(500));
        if now >= deadline {
            self.frame.recent_flush_deadline = None;
            self.settings.editor_config.flush_recent();
        } else {
            self.egui_ctx()
                .request_repaint_after(deadline.saturating_duration_since(now));
        }
    }
}
