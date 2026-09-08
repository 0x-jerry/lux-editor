//! Settings domain: the user configuration and the watcher that reloads it
//! when the file changes on disk.

use crate::events::{AppEvent, CustomEvent};
use crate::settings::Config;
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
