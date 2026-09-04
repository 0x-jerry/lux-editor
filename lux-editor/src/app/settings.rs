//! App-side reactions to configuration changes. Applying the appearance
//! (chrome theme, fonts) itself lives in `ui/theme.rs` — this module only
//! decides *when* to re-apply it.

use super::App;
use crate::config::{Config, EditorSettings};
use crate::events::{AppEvent, CustomEvent};
use eframe::egui;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

#[derive(Default)]
pub(crate) struct SettingsState {
    pub(crate) editor_config: Config,
    pub(crate) watcher: Option<RecommendedWatcher>,
}

impl SettingsState {
    pub(crate) fn start_watcher(
        watch_roots: &[PathBuf],
        event_tx: Sender<CustomEvent>,
        wake: egui::Context,
    ) -> Option<RecommendedWatcher> {
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<notify::Event>| {
                if result.is_ok() {
                    event_tx.send(CustomEvent::App(AppEvent::ConfigChange)).ok();
                    wake.request_repaint();
                }
            },
            notify::Config::default(),
        )
        .ok()?;

        for root in watch_roots {
            if !root.exists() {
                std::fs::create_dir_all(root).ok()?;
            }
            watcher.watch(root, RecursiveMode::NonRecursive).ok()?;
        }

        Some(watcher)
    }
}

impl App {
    pub(super) fn restart_settings_watcher(&mut self) {
        let watch_roots = Config::settings_watch_roots();
        self.settings.watcher = SettingsState::start_watcher(
            &watch_roots,
            self.runtime.event_tx.clone(),
            self.runtime.ctx.clone(),
        );
    }

    pub(super) fn apply_saved_configuration(&mut self, settings: EditorSettings) {
        let theme_changed = settings.theme != self.settings.editor_config.settings.theme;
        let font_changed = settings.font != self.settings.editor_config.settings.font;
        self.settings.editor_config.settings = settings;
        self.chrome.needs_style_refresh |= theme_changed || font_changed;
    }

    pub(super) fn on_config_change(&mut self) {
        if self.settings.editor_config.reload_settings() {
            self.chrome.needs_style_refresh = true;
        }
        self.chrome
            .shell
            .sync_config_draft(&self.settings.editor_config.settings);
    }
}
