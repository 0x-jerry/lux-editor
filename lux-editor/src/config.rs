use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RecentItem {
    pub path: PathBuf,
    pub is_dir: bool,
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
    pub settings: EditorSettings,
}

impl Config {
    pub fn load() -> Self {
        Self {
            recent_items: Self::load_recent_items(),
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
        self.save_recent_items();
    }

    pub fn user_settings_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lux")
            .join("config.toml")
    }

    pub fn save_settings(settings: &EditorSettings) -> std::io::Result<PathBuf> {
        let path = Self::user_settings_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, Self::serialize_settings(settings))?;
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

    fn load_recent_items() -> Vec<RecentItem> {
        if let Ok(data) = std::fs::read_to_string(Self::recent_items_path()) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    fn save_recent_items(&self) {
        let path = Self::recent_items_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(data) = serde_json::to_string(&self.recent_items) {
            std::fs::write(path, data).ok();
        }
    }

    fn recent_items_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lux")
            .join("recent.json")
    }

    fn serialize_settings(settings: &EditorSettings) -> String {
        let mut output = String::new();
        output.push_str("[theme]\n");
        output.push_str(&format!(
            "syntax_theme = \"{}\"\n",
            Self::escape_toml_string(&settings.theme.syntax_theme)
        ));
        if let Some(path) = &settings.theme.theme_path {
            output.push_str(&format!(
                "theme_path = \"{}\"\n",
                Self::escape_toml_string(&path.display().to_string())
            ));
        }
        output.push('\n');
        output.push_str("[font]\n");
        output.push_str(&format!(
            "family = \"{}\"\n",
            Self::escape_toml_string(&settings.font.family)
        ));
        output.push_str(&format!("size = {}\n", settings.font.size));
        output
    }

    fn escape_toml_string(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }
}
