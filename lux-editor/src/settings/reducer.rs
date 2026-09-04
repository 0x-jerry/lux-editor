//! Settings reducer: config-file refresh, recent-item clearing and the
//! configuration view's autosave.

use crate::app::App;
use crate::events::{AppEvent, ConfigurationEvent};
use eframe::egui;

impl App {
    /// App-level state & navigation: config refresh and open commands.
    pub(crate) fn handle_app_event(&mut self, event: AppEvent, ctx: &egui::Context) {
        match event {
            AppEvent::ConfigChange => self.on_config_change(),
            AppEvent::OpenFile(path) => self.open_file(path, ctx),
            AppEvent::OpenFolder(path) => self.open_folder(path, ctx),
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
}
