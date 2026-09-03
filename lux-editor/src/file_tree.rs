//! Workspace file-tree model: a gitignore-aware snapshot of the directory
//! tree. Pure data only — rendering lives in the `FileTreePanel` UI component
//! (`ui/components/file_tree.rs`).

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub enum Entry {
    File(PathBuf),
    Directory(PathBuf, Vec<Entry>),
}

pub struct FileTree {
    entry: Entry,
}

impl FileTree {
    pub fn new(path: &Path) -> Self {
        let ignored = Self::build_gitignore(path);
        Self {
            entry: Self::build_entry(path, &ignored),
        }
    }

    /// The tree root; the UI component walks it recursively.
    pub fn entry(&self) -> &Entry {
        &self.entry
    }

    fn build_entry(path: &Path, ignored: &Gitignore) -> Entry {
        if path.is_dir() {
            let mut entries = vec![];
            for entry in path.read_dir().expect("read_dir call failed").flatten() {
                let entry_path = entry.path();
                let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
                if ignored
                    .matched_path_or_any_parents(&entry_path, is_dir)
                    .is_ignore()
                {
                    continue;
                }
                entries.push(Self::build_entry(&entry_path, ignored));
            }
            entries.sort_by(|a, b| {
                let a_is_dir = matches!(a, Entry::Directory(_, _));
                let b_is_dir = matches!(b, Entry::Directory(_, _));
                if a_is_dir != b_is_dir {
                    return b_is_dir.cmp(&a_is_dir);
                }
                let a_name = Self::entry_name(a).to_lowercase();
                let b_name = Self::entry_name(b).to_lowercase();
                a_name.cmp(&b_name)
            });
            Entry::Directory(path.to_path_buf(), entries)
        } else {
            Entry::File(path.to_path_buf())
        }
    }

    fn build_gitignore(root: &Path) -> Gitignore {
        let mut builder = GitignoreBuilder::new(root);
        let gitignore_path = root.join(".gitignore");
        if gitignore_path.exists() {
            builder.add(gitignore_path);
        }
        builder.build().unwrap_or_else(|_| {
            let fallback = GitignoreBuilder::new(root);
            fallback.build().expect("gitignore builder must succeed")
        })
    }

    fn entry_name(entry: &Entry) -> String {
        match entry {
            Entry::File(path) => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            Entry::Directory(path, _) => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }
}
