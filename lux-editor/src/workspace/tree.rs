//! Workspace file-tree model: a gitignore-aware, lazily loaded snapshot of
//! the directory tree. Only directories the UI has asked for (root at open,
//! expanded folders after that) are read from disk. Gitignore-matched entries
//! stay in the snapshot flagged `ignored` so the panel can dim them; entries
//! matched by the configured exclude patterns are dropped. Pure data only —
//! rendering lives in the `FileTreePanel` component (`workspace::file_tree_panel`).

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub enum Entry {
    File { path: PathBuf, ignored: bool },
    Directory { path: PathBuf, ignored: bool },
}

impl Entry {
    fn name(&self) -> String {
        let path = match self {
            Entry::File { path, .. } | Entry::Directory { path, .. } => path,
        };
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
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
    /// The configured exclude patterns as one matcher; entries it matches are
    /// left out of the snapshot entirely (never dimmed, never scanned).
    exclude: Gitignore,
    exclude_patterns: Vec<String>,
    /// The patterns as configured, so an unchanged list short-circuits before
    /// `normalize_patterns` allocates on every logic pass.
    exclude_raw: Vec<String>,
}

impl FileTree {
    pub fn new(path: &Path, exclude: &[String]) -> Self {
        let exclude_patterns = normalize_patterns(exclude);
        let mut tree = Self {
            root: path.to_path_buf(),
            dirs: HashMap::new(),
            chains: HashMap::new(),
            exclude: exclude_matcher(path, &exclude_patterns),
            exclude_patterns,
            exclude_raw: exclude.to_vec(),
        };
        tree.load_dir(path);
        tree
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Point the tree at a new exclude list; a no-op while the patterns are
    /// unchanged, so the app can call it every frame. Returns whether the
    /// visible tree changed, so the caller can rebuild a watcher that captured
    /// the old patterns.
    pub fn set_excluded(&mut self, patterns: &[String]) -> bool {
        if self.exclude_raw == patterns {
            return false;
        }
        self.exclude_raw = patterns.to_vec();
        let normalized = normalize_patterns(patterns);
        if normalized == self.exclude_patterns {
            return false;
        }
        self.exclude = exclude_matcher(&self.root, &normalized);
        self.exclude_patterns = normalized;
        self.refresh();
        true
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
        let dir_ignored = self.dir_ignored(dir);
        let Ok(read) = std::fs::read_dir(dir) else {
            return false;
        };
        let mut entries = vec![];
        for entry in read.flatten() {
            let path = entry.path();
            let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            if self.exclude.matched(&path, is_dir).is_ignore() {
                continue;
            }
            // Everything below an ignored directory is ignored: git never
            // descends into one, so a nested whitelist cannot rescue it.
            let ignored = dir_ignored || Self::ignored(&chain, &path, is_dir);
            entries.push(if is_dir {
                Entry::Directory { path, ignored }
            } else {
                Entry::File { path, ignored }
            });
        }
        entries.sort_by(|a, b| {
            let a_is_dir = matches!(a, Entry::Directory { .. });
            let b_is_dir = matches!(b, Entry::Directory { .. });
            if a_is_dir != b_is_dir {
                return b_is_dir.cmp(&a_is_dir);
            }
            let a_name = a.name().to_lowercase();
            let b_name = b.name().to_lowercase();
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

    /// Whether `dir` itself is gitignored, decided by its ancestors' matchers
    /// only — a directory's own `.gitignore` cannot whitelist itself.
    fn dir_ignored(&mut self, dir: &Path) -> bool {
        let inside_root = |parent: &Path| parent == self.root || parent.starts_with(&self.root);
        let Some(parent) = dir.parent().filter(|parent| inside_root(parent)) else {
            return false;
        };
        let chain = self.chain_for(parent);
        Self::ignored(&chain, dir, true)
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

/// Blank patterns are the list editor's empty rows; dropping them keeps a
/// no-op edit from rebuilding the matcher and rescanning the tree.
pub(super) fn normalize_patterns(patterns: &[String]) -> Vec<String> {
    patterns
        .iter()
        .map(|pattern| pattern.trim())
        .filter(|pattern| !pattern.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The exclude patterns with gitignore semantics rooted at the workspace, so
/// `.git` matches at any depth and `/*.git` only the root.
fn exclude_matcher(root: &Path, patterns: &[String]) -> Gitignore {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        if let Err(err) = builder.add_line(None, pattern) {
            log::warn!("skipping exclude pattern {pattern:?}: {err}");
        }
    }
    // `add_line` already rejected every glob that fails to parse.
    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(name, ignored)` per row, in tree order.
    fn rows(children: &[Entry]) -> Vec<(String, bool)> {
        children
            .iter()
            .map(|entry| match entry {
                Entry::File { ignored, .. } | Entry::Directory { ignored, .. } => {
                    (entry.name(), *ignored)
                }
            })
            .collect()
    }

    fn row(name: &str, ignored: bool) -> (String, bool) {
        (name.to_string(), ignored)
    }

    #[test]
    fn lazy_tree_flags_nested_gitignore_and_keeps_dirs_first() {
        let root = tempfile::tempdir().unwrap();
        let root_path = root.path();
        std::fs::write(root_path.join(".gitignore"), "node_modules\nz.rs\n*.txt\n").unwrap();
        std::fs::create_dir(root_path.join("node_modules")).unwrap();
        std::fs::write(root_path.join("node_modules/lib.js"), "").unwrap();
        std::fs::write(root_path.join("z.rs"), "").unwrap();
        std::fs::write(root_path.join("a.rs"), "").unwrap();
        std::fs::write(root_path.join("notes.txt"), "").unwrap();
        std::fs::create_dir(root_path.join("sub")).unwrap();
        std::fs::write(
            root_path.join("sub/.gitignore"),
            "secret.txt\n!visible.txt\n",
        )
        .unwrap();
        std::fs::write(root_path.join("sub/secret.txt"), "").unwrap();
        std::fs::write(root_path.join("sub/visible.txt"), "").unwrap();

        let mut tree = FileTree::new(root_path, &[]);
        // Dirs sort before files, case-insensitively; gitignored entries stay
        // in the snapshot flagged, so the panel can dim them.
        assert_eq!(
            rows(&tree.children(root_path)),
            vec![
                row("node_modules", true),
                row("sub", false),
                row(".gitignore", false),
                row("a.rs", false),
                row("notes.txt", true),
                row("z.rs", true),
            ]
        );
        // Only the root was scanned until a directory is asked for.
        assert!(!tree.dirs.contains_key(&root_path.join("sub")));
        // Deepest gitignore wins: the root ignores `*.txt`, sub re-includes
        // `visible.txt` while still flagging `secret.txt`.
        assert_eq!(
            rows(&tree.children(&root_path.join("sub"))),
            vec![
                row(".gitignore", false),
                row("secret.txt", true),
                row("visible.txt", false),
            ]
        );
        // Below an ignored directory every entry inherits the flag, even with
        // no matcher naming it.
        assert_eq!(
            rows(&tree.children(&root_path.join("node_modules"))),
            vec![row("lib.js", true)]
        );

        // refresh() re-reads the cached levels and picks up new files.
        std::fs::write(root_path.join("b.rs"), "").unwrap();
        std::fs::remove_file(root_path.join("a.rs")).unwrap();
        tree.refresh();
        assert_eq!(
            rows(&tree.children(root_path)),
            vec![
                row("node_modules", true),
                row("sub", false),
                row(".gitignore", false),
                row("b.rs", false),
                row("notes.txt", true),
                row("z.rs", true),
            ]
        );
    }

    #[test]
    fn exclude_patterns_hide_entries_at_any_depth_until_changed() {
        let root = tempfile::tempdir().unwrap();
        let root_path = root.path();
        std::fs::create_dir(root_path.join(".git")).unwrap();
        std::fs::write(root_path.join(".git/index"), "").unwrap();
        std::fs::create_dir_all(root_path.join("sub/.git")).unwrap();
        std::fs::write(root_path.join("keep.rs"), "").unwrap();

        let mut tree = FileTree::new(root_path, &[".git".to_string()]);
        assert_eq!(
            rows(&tree.children(root_path)),
            vec![row("sub", false), row("keep.rs", false)]
        );
        // An excluded directory is never scanned.
        assert!(!tree.dirs.contains_key(&root_path.join(".git")));

        assert!(tree.set_excluded(&[]));
        assert_eq!(
            rows(&tree.children(root_path)),
            vec![row(".git", false), row("sub", false), row("keep.rs", false)]
        );
        assert_eq!(
            rows(&tree.children(&root_path.join("sub"))),
            vec![row(".git", false)]
        );

        // A blank pattern (the list editor's empty row) hides nothing, and a
        // bare name reaches nested repositories too.
        assert!(tree.set_excluded(&[String::new(), ".git".to_string()]));
        // A respelled but equivalent list is a no-op, so no rescan happens.
        assert!(!tree.set_excluded(&[" .git ".to_string()]));
        assert_eq!(
            rows(&tree.children(root_path)),
            vec![row("sub", false), row("keep.rs", false)]
        );
        assert_eq!(rows(&tree.children(&root_path.join("sub"))), vec![]);
    }
}
