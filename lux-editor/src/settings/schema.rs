//! Serde schema of everything the user can configure, plus the recent-items
//! record shapes persisted alongside it (see [`super::store`]).

use std::path::PathBuf;

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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct BehaviorSettings {
    /// Auto-close brackets/quotes, skip over closing partners, and delete
    /// auto-paired pairs with Backspace.
    pub smart_pairing: bool,
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            smart_pairing: true,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_theme_settings_ignore_removed_fields() {
        let settings: ThemeSettings =
            serde_json::from_str(r#"{"syntax_theme":"InspiredGitHub","theme_path":null}"#).unwrap();
        assert_eq!(settings.choice, "auto");
    }

    #[test]
    fn theme_settings_default_choice_is_auto() {
        assert_eq!(ThemeSettings::default().choice, "auto");
    }
}
