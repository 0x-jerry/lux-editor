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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct ThemeSettings {
    pub syntax_theme: String,
    pub theme_path: Option<PathBuf>,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
pub struct EditorSettings {
    pub theme: ThemeSettings,
    pub font: FontSettings,
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
        self.recent_items.retain(|i| i.path != item.path);
        self.recent_items.insert(0, item);
        if self.recent_items.len() > 10 {
            self.recent_items.truncate(10);
        }
        self.prune_workspace_file_states_to_recent_dirs();
        self.save_recent_config();
    }

    pub fn clear_recent_items(&mut self) {
        self.recent_items.clear();
        self.workspace_file_states.clear();
        self.save_recent_config();
    }

    pub fn set_workspace_last_file(&mut self, workspace_path: &Path, file_path: &Path) {
        self.workspace_file_states
            .retain(|entry| entry.workspace_path != workspace_path);
        self.workspace_file_states.insert(
            0,
            WorkspaceFileState {
                workspace_path: workspace_path.to_path_buf(),
                file_path: file_path.to_path_buf(),
            },
        );
        if self.workspace_file_states.len() > 50 {
            self.workspace_file_states.truncate(50);
        }
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
            .set_default("theme.syntax_theme", "base16-ocean.dark")
            .unwrap()
            .set_default("font.family", "JetBrains Mono")
            .unwrap()
            .set_default("font.size", 14.0)
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
