use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc::UnboundedReceiver;

pub fn watch<P: AsRef<Path>>(
    path: P,
) -> notify::Result<(
    RecommendedWatcher,
    UnboundedReceiver<notify::Result<notify::Event>>,
)> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        notify::Config::default(),
    )?;

    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    Ok((watcher, rx))
}
