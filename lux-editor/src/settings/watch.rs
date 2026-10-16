//! Settings domain: the user configuration and the watcher that reloads it
//! when the file changes on disk.

use crate::events::{AppEvent, CustomEvent};
use crate::settings::Config;
use crate::wake::Wake;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::Sender;

/// Watch the directory holding the settings file. The returned watcher must be
/// kept alive for as long as the app wants reloads.
pub(crate) fn watch(event_tx: Sender<CustomEvent>, wake: Wake) -> Option<RecommendedWatcher> {
    let mut watcher = RecommendedWatcher::new(
        move |result: notify::Result<notify::Event>| {
            let Ok(event) = result else { return };
            // The log file lives in the watched settings directory; writing it
            // must not look like a config change (trace logging would otherwise
            // reload the config and repaint on every line).
            if !event.paths.is_empty()
                && event
                    .paths
                    .iter()
                    .all(|path| crate::logging::is_log_file(path))
            {
                return;
            }
            event_tx.send(CustomEvent::App(AppEvent::ConfigChange)).ok();
            wake.request_repaint();
        },
        notify::Config::default(),
    )
    .ok()?;

    for root in Config::settings_watch_roots() {
        if !root.exists() {
            std::fs::create_dir_all(&root).ok()?;
        }
        watcher.watch(&root, RecursiveMode::NonRecursive).ok()?;
    }

    Some(watcher)
}
