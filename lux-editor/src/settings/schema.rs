//! Serde schema of everything the user can configure, plus the recent-items
//! record shapes persisted alongside it (see [`super::store`]).

use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RecentItem {
    pub path: PathBuf,
    pub is_dir: bool,
}

/// What the editor remembers about a workspace: tabs in order, focused tab,
/// expanded tree folders.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
pub struct WorkspaceSession {
    #[serde(default)]
    pub workspace_path: PathBuf,
    #[serde(default)]
    pub open_files: Vec<PathBuf>,
    #[serde(default)]
    pub active_file: Option<PathBuf>,
    /// The configuration tab was focused when the session was saved; reopening
    /// the workspace restores it as the active tab.
    #[serde(default)]
    pub configuration_open: bool,
    #[serde(default)]
    pub expanded_dirs: Vec<PathBuf>,
}

fn default_theme_choice() -> String {
    "auto".to_string()
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct ThemeSettings {
    /// App theme: "auto" | "dark" | "light". `Auto` follows the OS.
    /// Chrome and syntax colors both come from the matching theme file
    /// (`assets/themes/*.json`, embedded at compile time).
    #[serde(default = "default_theme_choice")]
    pub choice: String,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            choice: default_theme_choice(),
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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
pub struct EditorSettings {
    pub theme: ThemeSettings,
    pub font: FontSettings,
    #[serde(default)]
    pub formatter: FormatterSettings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_theme_settings_ignore_removed_fields() {
        let settings: ThemeSettings =
            serde_json::from_str(r#"{"syntax_theme":"InspiredGitHub","theme_path":null}"#).unwrap();
        assert_eq!(settings.choice, "auto");
    }
}
