use super::App;
use crate::events::CustomEvent;
use crate::file_watcher;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

impl App {
    pub(super) fn start_workspace_watcher(
        workspace_path: &Path,
        event_tx: Sender<CustomEvent>,
    ) -> Option<RecommendedWatcher> {
        if let Ok((watcher, rx)) = file_watcher::watch(workspace_path) {
            std::thread::spawn(move || {
                let debounce = Duration::from_millis(120);
                while let Ok(result) = rx.recv() {
                    if result.is_err() {
                        continue;
                    }
                    let batch_start = Instant::now();
                    while batch_start.elapsed() < debounce {
                        let timeout = debounce.saturating_sub(batch_start.elapsed());
                        if rx.recv_timeout(timeout).is_err() {
                            break;
                        }
                    }
                    event_tx.send(CustomEvent::FileChange).ok();
                }
            });
            Some(watcher)
        } else {
            None
        }
    }

    pub(super) fn start_settings_watcher(
        watch_roots: &[PathBuf],
        event_tx: Sender<CustomEvent>,
    ) -> Option<RecommendedWatcher> {
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<notify::Event>| {
                if result.is_ok() {
                    event_tx.send(CustomEvent::ConfigChange).ok();
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
