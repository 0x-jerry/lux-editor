//! On-disk settings store: user `config.json`, `recent.json` (recent items +
//! per-workspace sessions: open tabs, active file, expanded tree folders) and
//! the in-memory [`Config`] that debounces recent-item writes.

use super::schema::{EditorSettings, RecentItem, WorkspaceSession};
use std::path::{Path, PathBuf};

const MAX_SESSIONS: usize = 50;
const MAX_SESSION_FILES: usize = 100;
const MAX_EXPANDED_DIRS: usize = 512;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
struct RecentConfigFile {
    #[serde(default)]
    recent_items: Vec<RecentItem>,
    #[serde(default)]
    workspace_sessions: Vec<WorkspaceSession>,
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub recent_items: Vec<RecentItem>,
    workspace_sessions: Vec<WorkspaceSession>,
    pub settings: EditorSettings,
    /// Recent changes wait for the app's debounced flush instead of writing
    /// synchronously (including during startup).
    pub recent_dirty: bool,
}

impl Config {
    pub fn load() -> Self {
        let recent_config = Self::load_recent_config();
        Self {
            recent_items: recent_config.recent_items,
            workspace_sessions: recent_config.workspace_sessions,
            settings: Self::load_settings(),
            recent_dirty: false,
        }
    }

    /// Write pending recent-items changes; safe to call every frame. On a
    /// failed write the flag stays set so the app's loop retries.
    pub fn flush_recent(&mut self) {
        if !self.recent_dirty {
            return;
        }
        if self.save_recent_config() {
            self.recent_dirty = false;
        }
    }

    pub fn reload_settings(&mut self) -> bool {
        let next = Self::load_settings();
        if self.settings == next {
            return false;
        }
        self.settings = next;
        true
    }

    pub fn settings_watch_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Some(parent) = Self::user_settings_path().parent() {
            roots.push(parent.to_path_buf());
        }
        roots
    }

    pub fn add_recent(&mut self, path: PathBuf, is_dir: bool) {
        let item = RecentItem { path, is_dir };
        insert_recent(&mut self.recent_items, item, 10);
        self.recent_dirty = true;
    }

    pub fn clear_recent_items(&mut self) {
        self.recent_items.clear();
        self.workspace_sessions.clear();
        self.recent_dirty = true;
    }

    pub fn workspace_session(&self, workspace_path: &Path) -> Option<&WorkspaceSession> {
        self.workspace_sessions
            .iter()
            .find(|session| session.workspace_path == workspace_path)
    }

    /// Upsert a workspace session; `false` when nothing changed, so the per-
    /// frame session snapshot never re-arms the debounced disk write. Bounded by
    /// `MAX_SESSIONS`, and not tied to `recent_items`, which file opens drain.
    pub fn set_workspace_session(&mut self, mut session: WorkspaceSession) -> bool {
        session
            .open_files
            .truncate(session.open_files.len().min(MAX_SESSION_FILES));
        session.expanded_dirs.sort();
        session
            .expanded_dirs
            .truncate(session.expanded_dirs.len().min(MAX_EXPANDED_DIRS));
        let existing = self
            .workspace_sessions
            .iter()
            .position(|stored| stored.workspace_path == session.workspace_path);
        match existing {
            Some(index) if self.workspace_sessions[index] == session => return false,
            Some(index) => {
                self.workspace_sessions[index] = session;
            }
            None => {
                self.workspace_sessions.insert(0, session);
                self.workspace_sessions.truncate(MAX_SESSIONS);
            }
        }
        self.recent_dirty = true;
        true
    }

    pub fn user_settings_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lux")
            .join("config.json")
    }

    pub fn save_settings(settings: &EditorSettings) -> std::io::Result<PathBuf> {
        let path = Self::user_settings_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(settings)?;
        std::fs::write(&path, content)?;
        Ok(path)
    }

    pub(crate) fn load_settings() -> EditorSettings {
        let user_settings = Self::user_settings_path();
        ::config::Config::builder()
            .set_default("theme.choice", "auto")
            .unwrap()
            .set_default("font.family", "JetBrains Mono")
            .unwrap()
            .set_default("font.size", 14.0)
            .unwrap()
            .set_default("formatter.command", "")
            .unwrap()
            .set_default("formatter.args", "--stdin")
            .unwrap()
            .set_default("formatter.format_on_save", true)
            .unwrap()
            .set_default("behavior.smart_pairing", true)
            .unwrap()
            .add_source(::config::File::from(user_settings).required(false))
            .build()
            .ok()
            .and_then(|cfg| cfg.try_deserialize::<EditorSettings>().ok())
            .unwrap_or_default()
    }

    fn load_recent_config() -> RecentConfigFile {
        let path = Self::recent_items_path();
        let Ok(data) = std::fs::read_to_string(&path) else {
            return RecentConfigFile::default();
        };
        match serde_json::from_str::<RecentConfigFile>(&data) {
            Ok(mut file) => {
                // A truncated entry would park a session nothing can look up.
                file.workspace_sessions
                    .retain(|session| !session.workspace_path.as_os_str().is_empty());
                file
            }
            Err(err) => {
                // The next flush would replace the file; keep the bytes readable.
                log::warn!("unreadable {}, keeping a copy beside it: {err}", path.display());
                std::fs::rename(&path, path.with_extension("json.bak")).ok();
                RecentConfigFile::default()
            }
        }
    }

    fn save_recent_config(&self) -> bool {
        let path = Self::recent_items_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let data = RecentConfigFile {
            recent_items: self.recent_items.clone(),
            workspace_sessions: self.workspace_sessions.clone(),
        };
        match serde_json::to_string(&data) {
            Ok(data) => std::fs::write(path, data).is_ok(),
            Err(_) => false,
        }
    }

    fn recent_items_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lux")
            .join("recent.json")
    }
}

fn insert_recent(items: &mut Vec<RecentItem>, item: RecentItem, max: usize) {
    items.retain(|existing| existing.path != item.path);
    items.insert(0, item);
    items.truncate(max);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recent(path: &str, is_dir: bool) -> RecentItem {
        RecentItem {
            path: PathBuf::from(path),
            is_dir,
        }
    }

    #[test]
    fn insert_recent_dedups_and_caps() {
        let mut items = vec![recent("/a", false)];
        insert_recent(&mut items, recent("/a", false), 10);
        assert_eq!(items.len(), 1);

        for i in 0..15 {
            insert_recent(&mut items, recent(&format!("/f{i}"), false), 10);
        }
        assert_eq!(items.len(), 10);
        assert_eq!(items[0].path, PathBuf::from("/f14"));
    }

    fn session(workspace: &str, files: &[&str], expanded: &[&str]) -> WorkspaceSession {
        WorkspaceSession {
            workspace_path: PathBuf::from(workspace),
            open_files: files.iter().map(PathBuf::from).collect(),
            active_file: files.last().map(PathBuf::from),
            expanded_dirs: expanded.iter().map(PathBuf::from).collect(),
        }
    }

    #[test]
    fn set_workspace_session_upserts_and_detects_no_change() {

        let mut config = Config {
            recent_items: vec![recent("/ws", true)],
            ..Default::default()
        };
        assert!(config.set_workspace_session(session("/ws", &["/ws/a.rs"], &["/ws/b"])));
        assert!(config.recent_dirty);
        config.recent_dirty = false;
        // Identical snapshot (order-insensitive expansion) must not re-dirty.
        assert!(!config.set_workspace_session(session("/ws", &["/ws/a.rs"], &["/ws/b"])));
        assert!(!config.recent_dirty);
        assert!(config.set_workspace_session(session("/ws", &["/ws/a.rs"], &[])));
        assert_eq!(
            config
                .workspace_session(Path::new("/ws"))
                .unwrap()
                .open_files,
            vec![PathBuf::from("/ws/a.rs")]
        );

        let many = (0..150)
            .map(|index| format!("/ws/f{index}"))
            .collect::<Vec<_>>();
        let refs = many.iter().map(String::as_str).collect::<Vec<_>>();
        config.set_workspace_session(session("/ws", &refs, &[]));
        assert_eq!(
            config.workspace_session(Path::new("/ws")).unwrap().open_files.len(),
            MAX_SESSION_FILES
        );
    }

    #[test]
    fn session_survives_recent_eviction() {
        let mut config = Config::default();
        config.set_workspace_session(session("/ws", &["/ws/f"], &[]));
        for index in 0..12 {
            config.add_recent(PathBuf::from(format!("/other{index}.rs")), false);
        }
        assert_eq!(config.recent_items.len(), 10);
        assert!(config.workspace_session(Path::new("/ws")).is_some());
    }

    #[test]
    fn recent_config_fields_are_all_optional() {
        let file: RecentConfigFile = serde_json::from_str(r#"{"recent_items":[]}"#).unwrap();
        assert!(file.workspace_sessions.is_empty());
    }
}
