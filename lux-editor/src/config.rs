use std::path::{Path, PathBuf};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RecentItem {
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct WorkspaceFileState {
    pub workspace_path: PathBuf,
    pub file_path: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
struct RecentConfigFile {
    recent_items: Vec<RecentItem>,
    workspace_file_states: Vec<WorkspaceFileState>,
}

fn default_theme_choice() -> String {
    "auto".to_string()
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct ThemeSettings {
    /// App chrome theme: "auto" | "dark" | "light". `Auto` follows the OS.
    #[serde(default = "default_theme_choice")]
    pub choice: String,
    pub syntax_theme: String,
    pub theme_path: Option<PathBuf>,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            choice: "auto".to_string(),
            syntax_theme: "base16-ocean.dark".to_string(),
            theme_path: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct FontSettings {
    pub family: String,
    pub size: f32,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            family: "JetBrains Mono".to_string(),
            size: 14.0,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct FormatterSettings {
    /// External command run with the document text on stdin; empty disables.
    pub command: String,
    /// Space-separated arguments passed to the command.
    pub args: String,
    /// Format the buffer immediately before saving.
    pub format_on_save: bool,
}

impl Default for FormatterSettings {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: "--stdin".to_string(),
            format_on_save: true,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct BehaviorSettings {
    /// Auto-close brackets/quotes, skip over closing partners, and delete
    /// auto-paired pairs with Backspace.
    pub smart_pairing: bool,
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self { smart_pairing: true }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
pub struct EditorSettings {
    pub theme: ThemeSettings,
    pub font: FontSettings,
    #[serde(default)]
    pub formatter: FormatterSettings,
    #[serde(default)]
    pub behavior: BehaviorSettings,
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub recent_items: Vec<RecentItem>,
    workspace_file_states: Vec<WorkspaceFileState>,
    pub settings: EditorSettings,
}

impl Config {
    pub fn load() -> Self {
        let recent_config = Self::load_recent_config();
        Self {
            recent_items: recent_config.recent_items,
            workspace_file_states: recent_config.workspace_file_states,
            settings: Self::load_settings(),
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
        self.prune_workspace_file_states_to_recent_dirs();
        self.save_recent_config();
    }

    pub fn clear_recent_items(&mut self) {
        self.recent_items.clear();
        self.workspace_file_states.clear();
        self.save_recent_config();
    }

    pub fn set_workspace_last_file(&mut self, workspace_path: &Path, file_path: &Path) {
        insert_workspace_state(
            &mut self.workspace_file_states,
            WorkspaceFileState {
                workspace_path: workspace_path.to_path_buf(),
                file_path: file_path.to_path_buf(),
            },
            50,
        );
        self.prune_workspace_file_states_to_recent_dirs();
        self.save_recent_config();
    }

    pub fn workspace_last_file(&self, workspace_path: &Path) -> Option<PathBuf> {
        self.workspace_file_states
            .iter()
            .find(|entry| entry.workspace_path == workspace_path)
            .map(|entry| entry.file_path.clone())
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

    fn load_settings() -> EditorSettings {
        let user_settings = Self::user_settings_path();
        ::config::Config::builder()
            .set_default("theme.choice", "auto")
            .unwrap()
            .set_default("theme.syntax_theme", "base16-ocean.dark")
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
        if let Ok(data) = std::fs::read_to_string(Self::recent_items_path()) {
            serde_json::from_str::<RecentConfigFile>(&data).unwrap_or_default()
        } else {
            RecentConfigFile::default()
        }
    }

    fn save_recent_config(&self) {
        let path = Self::recent_items_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let data = RecentConfigFile {
            recent_items: self.recent_items.clone(),
            workspace_file_states: self.workspace_file_states.clone(),
        };
        if let Ok(data) = serde_json::to_string(&data) {
            std::fs::write(path, data).ok();
        }
    }

    fn prune_workspace_file_states_to_recent_dirs(&mut self) {
        self.workspace_file_states.retain(|workspace_state| {
            self.recent_items
                .iter()
                .any(|item| item.is_dir && item.path == workspace_state.workspace_path)
        });
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

fn insert_workspace_state(
    states: &mut Vec<WorkspaceFileState>,
    state: WorkspaceFileState,
    max: usize,
) {
    states.retain(|existing| existing.workspace_path != state.workspace_path);
    states.insert(0, state);
    states.truncate(max);
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

    #[test]
    fn insert_workspace_state_dedups_and_caps() {
        let mut states = Vec::new();
        for i in 0..60 {
            insert_workspace_state(
                &mut states,
                WorkspaceFileState {
                    workspace_path: PathBuf::from(format!("/ws{i}")),
                    file_path: PathBuf::from("/f"),
                },
                50,
            );
        }
        assert_eq!(states.len(), 50);
        assert_eq!(states[0].workspace_path, PathBuf::from("/ws59"));
    }

    #[test]
    fn prune_workspace_states_keeps_only_recent_dirs() {
        let mut config = Config {
            recent_items: vec![recent("/ws1", true)],
            workspace_file_states: vec![
                WorkspaceFileState {
                    workspace_path: PathBuf::from("/ws1"),
                    file_path: PathBuf::from("/ws1/f"),
                },
                WorkspaceFileState {
                    workspace_path: PathBuf::from("/gone"),
                    file_path: PathBuf::from("/gone/f"),
                },
            ],
            settings: Default::default(),
        };
        config.prune_workspace_file_states_to_recent_dirs();
        assert_eq!(config.workspace_file_states.len(), 1);
        assert_eq!(
            config.workspace_file_states[0].workspace_path,
            PathBuf::from("/ws1")
        );
    }

    #[test]
    fn legacy_theme_settings_without_choice_parse_as_auto() {
        let settings: ThemeSettings =
            serde_json::from_str(r#"{"syntax_theme":"InspiredGitHub","theme_path":null}"#)
                .unwrap();
        assert_eq!(settings.choice, "auto");
        assert_eq!(settings.syntax_theme, "InspiredGitHub");
    }

    #[test]
    fn theme_settings_default_choice_is_auto() {
        assert_eq!(ThemeSettings::default().choice, "auto");
    }

    #[test]
    fn workspace_last_file_finds_matching_entry() {
        let config = Config {
            recent_items: vec![],
            workspace_file_states: vec![WorkspaceFileState {
                workspace_path: PathBuf::from("/ws"),
                file_path: PathBuf::from("/ws/a.rs"),
            }],
            settings: Default::default(),
        };
        assert_eq!(
            config.workspace_last_file(Path::new("/ws")),
            Some(PathBuf::from("/ws/a.rs"))
        );
        assert_eq!(config.workspace_last_file(Path::new("/other")), None);
    }
}
