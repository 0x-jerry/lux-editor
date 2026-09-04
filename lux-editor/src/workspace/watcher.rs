use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::Receiver;

pub fn watch<P: AsRef<Path>>(
    path: P,
) -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let root_path = path.as_ref().to_path_buf();
    let ignored = build_gitignore(&root_path);
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                if !is_watch_event_relevant(&event) {
                    return;
                }
                if event.paths.iter().all(|path| is_ignored(path, &ignored)) {
                    return;
                }
                let _ = tx.send(Ok(event));
                return;
            }
            let _ = tx.send(res);
        },
        notify::Config::default(),
    )?;

    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    Ok((watcher, rx))
}

fn build_gitignore(workspace_root: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(workspace_root);
    let gitignore_path = workspace_root.join(".gitignore");
    if gitignore_path.exists() {
        builder.add(gitignore_path);
    }
    builder.build().unwrap_or_else(|_| {
        let fallback = GitignoreBuilder::new(workspace_root);
        fallback.build().expect("gitignore builder must succeed")
    })
}

fn is_watch_event_relevant(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_)
            | EventKind::Modify(_)
            | EventKind::Remove(_)
            | EventKind::Any
            | EventKind::Other
    )
}

fn is_ignored(path: &Path, matcher: &Gitignore) -> bool {
    let is_dir = path.is_dir();
    matcher
        .matched_path_or_any_parents(path, is_dir)
        .is_ignore()
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, AccessMode, CreateKind, DataChange, ModifyKind};
    use std::fs;

    #[test]
    fn gitignore_filters_ignored_paths() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), "target\n*.log\n").unwrap();
        fs::create_dir(dir.path().join("target")).unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("target/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("debug.log"), "x").unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

        let ignored = build_gitignore(dir.path());
        assert!(is_ignored(&dir.path().join("target/main.rs"), &ignored));
        assert!(is_ignored(&dir.path().join("debug.log"), &ignored));
        assert!(!is_ignored(&dir.path().join("src/main.rs"), &ignored));
    }

    #[test]
    fn watch_event_relevance() {
        assert!(is_watch_event_relevant(&notify::Event::new(
            EventKind::Create(CreateKind::File)
        )));
        assert!(is_watch_event_relevant(&notify::Event::new(
            EventKind::Modify(ModifyKind::Data(DataChange::Any))
        )));
        assert!(!is_watch_event_relevant(&notify::Event::new(
            EventKind::Access(AccessKind::Close(AccessMode::Any))
        )));
    }
}
