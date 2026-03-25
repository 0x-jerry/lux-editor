#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RecentItem {
    pub path: std::path::PathBuf,
    pub is_dir: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Config {
    pub recent_items: Vec<RecentItem>,
}

impl Config {
    pub fn load() -> Self {
        let path = Self::path();
        if let Ok(data) = std::fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn add_recent(&mut self, path: std::path::PathBuf, is_dir: bool) {
        let item = RecentItem { path, is_dir };
        self.recent_items.retain(|i| i.path != item.path);
        self.recent_items.insert(0, item);
        if self.recent_items.len() > 10 {
            self.recent_items.truncate(10);
        }
        self.save();
    }

    fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(data) = serde_json::to_string(self) {
            std::fs::write(path, data).ok();
        }
    }

    fn path() -> std::path::PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("lux")
            .join("config.json")
    }
}
