//! Workspace file-tree model: a gitignore-aware, lazily loaded snapshot of
//! the directory tree. Only directories the UI has asked for (root at open,
//! expanded folders after that) are read from disk. Pure data only —
//! rendering lives in the `FileTreePanel` component (`workspace::ui`).

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub enum Entry {
    File(PathBuf),
    Directory(PathBuf),
}

/// One level of `.gitignore` rules plus the parent level, so a directory's
/// effective matchers are the chain of `.gitignore` files from the root down.
struct IgnoreChain {
    matcher: Option<Gitignore>,
    parent: Option<Arc<IgnoreChain>>,
}

pub struct FileTree {
    root: PathBuf,
    dirs: HashMap<PathBuf, Arc<Vec<Entry>>>,
    chains: HashMap<PathBuf, Arc<IgnoreChain>>,
}

impl FileTree {
    pub fn new(path: &Path) -> Self {
        let mut tree = Self {
            root: path.to_path_buf(),
            dirs: HashMap::new(),
            chains: HashMap::new(),
        };
        tree.load_dir(path);
        tree
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Sorted children of `dir`, loaded from disk on first request.
    pub fn children(&mut self, dir: &Path) -> Arc<Vec<Entry>> {
        if let Some(children) = self.dirs.get(dir) {
            return Arc::clone(children);
        }
        if self.load_dir(dir)
            && let Some(children) = self.dirs.get(dir)
        {
            return Arc::clone(children);
        }
        Arc::new(Vec::new())
    }

    /// Reload every directory loaded so far; drives the file-watcher refresh
    /// without rescanning what the UI never expanded.
    pub fn refresh(&mut self) {
        let cached: Vec<PathBuf> = self.dirs.keys().cloned().collect();
        self.dirs.clear();
        self.chains.clear();
        for dir in cached {
            if dir.is_dir() {
                self.load_dir(&dir);
            }
        }
    }

    /// `false` when the directory could not be read; failures are not cached
    /// so a transient error is retried on the next request.
    fn load_dir(&mut self, dir: &Path) -> bool {
        let chain = self.chain_for(dir);
        let Ok(read) = std::fs::read_dir(dir) else {
            return false;
        };
        let mut entries = vec![];
        for entry in read.flatten() {
            let path = entry.path();
            let is_dir = entry
                .file_type()
                .map(|kind| kind.is_dir())
                .unwrap_or(false);
            if Self::ignored(&chain, &path, is_dir) {
                continue;
            }
            entries.push(if is_dir {
                Entry::Directory(path)
            } else {
                Entry::File(path)
            });
        }
        entries.sort_by(|a, b| {
            let a_is_dir = matches!(a, Entry::Directory(_));
            let b_is_dir = matches!(b, Entry::Directory(_));
            if a_is_dir != b_is_dir {
                return b_is_dir.cmp(&a_is_dir);
            }
            let a_name = Self::entry_name(a).to_lowercase();
            let b_name = Self::entry_name(b).to_lowercase();
            a_name.cmp(&b_name)
        });
        self.dirs.insert(dir.to_path_buf(), Arc::new(entries));
        true
    }

    fn chain_for(&mut self, dir: &Path) -> Arc<IgnoreChain> {
        if let Some(chain) = self.chains.get(dir) {
            return Arc::clone(chain);
        }
        // Stops at the workspace root: anything above it never contributes
        // ignore files, and paths outside root degrade to rootless (own
        // `.gitignore` only) instead of climbing to `/`.
        let parent = match dir.parent() {
            Some(parent) if parent == self.root || parent.starts_with(&self.root) => {
                Some(self.chain_for(parent))
            }
            _ => None,
        };
        let matcher = dir_gitignore(dir);
        let chain = Arc::new(IgnoreChain { matcher, parent });
        self.chains.insert(dir.to_path_buf(), Arc::clone(&chain));
        chain
    }

    /// Deepest `.gitignore` wins (git semantics): walk root→leaf, the last
    /// matcher with an opinion (ignore or whitelist) decides.
    fn ignored(chain: &IgnoreChain, path: &Path, is_dir: bool) -> bool {
        fn walk(node: &IgnoreChain, path: &Path, is_dir: bool, verdict: &mut bool) {
            if let Some(parent) = node.parent.as_deref() {
                walk(parent, path, is_dir, verdict);
            }
            if let Some(matcher) = &node.matcher {
                let matched = matcher.matched(path, is_dir);
                if matched.is_ignore() {
                    *verdict = true;
                } else if matched.is_whitelist() {
                    *verdict = false;
                }
            }
        }
        let mut verdict = false;
        walk(chain, path, is_dir, &mut verdict);
        verdict
    }

    fn entry_name(entry: &Entry) -> String {
        let path = match entry {
            Entry::File(path) | Entry::Directory(path) => path,
        };
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

fn dir_gitignore(dir: &Path) -> Option<Gitignore> {
    let gitignore_path = dir.join(".gitignore");
    if !gitignore_path.exists() {
        return None;
    }
    let mut builder = GitignoreBuilder::new(dir);
    builder.add(gitignore_path);
    builder.build().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(children: &[Entry]) -> Vec<String> {
        children
            .iter()
            .map(|entry| match entry {
                Entry::File(path) | Entry::Directory(path) => {
                    path.file_name().unwrap().to_string_lossy().into_owned()
                }
            })
            .collect()
    }

    #[test]
    fn lazy_tree_honors_nested_gitignore_and_dirs_first() {
        let root = tempfile::tempdir().unwrap();
        let root_path = root.path();
        std::fs::write(root_path.join(".gitignore"), "node_modules\nz.rs\n*.txt\n").unwrap();
        std::fs::create_dir(root_path.join("node_modules")).unwrap();
        std::fs::write(root_path.join("node_modules/lib.js"), "").unwrap();
        std::fs::write(root_path.join("z.rs"), "").unwrap();
        std::fs::write(root_path.join("a.rs"), "").unwrap();
        std::fs::write(root_path.join("notes.txt"), "").unwrap();
        std::fs::create_dir(root_path.join("sub")).unwrap();
        std::fs::write(root_path.join("sub/.gitignore"), "secret.txt\n!visible.txt\n").unwrap();
        std::fs::write(root_path.join("sub/secret.txt"), "").unwrap();
        std::fs::write(root_path.join("sub/visible.txt"), "").unwrap();

        let mut tree = FileTree::new(root_path);
        // Dirs sort before files, case-insensitively; root-level ignores apply
        // (notes.txt falls to `*.txt`).
        assert_eq!(
            names(&tree.children(root_path)),
            vec!["sub", ".gitignore", "a.rs"]
        );
        // Only the root was scanned until a directory is asked for.
        assert!(!tree.dirs.contains_key(&root_path.join("sub")));
        // Deepest gitignore wins: the root ignores `*.txt`, sub re-includes
        // `visible.txt` while still hiding `secret.txt`.
        assert_eq!(
            names(&tree.children(&root_path.join("sub"))),
            vec![".gitignore", "visible.txt"]
        );

        // refresh() re-reads the cached levels and picks up new files.
        std::fs::write(root_path.join("b.rs"), "").unwrap();
        std::fs::remove_file(root_path.join("a.rs")).unwrap();
        tree.refresh();
        assert_eq!(
            names(&tree.children(root_path)),
            vec!["sub", ".gitignore", "b.rs"]
        );
    }
}
