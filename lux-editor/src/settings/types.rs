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

fn default_font_family() -> String {
    "JetBrains Mono".to_string()
}

fn default_font_size() -> f32 {
    14.0
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct FontSettings {
    #[serde(default = "default_font_family")]
    pub family: String,
    #[serde(default = "default_font_size")]
    pub size: f32,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            family: default_font_family(),
            size: default_font_size(),
        }
    }
}

fn default_formatter_command() -> String {
    String::new()
}

fn default_formatter_args() -> String {
    "--stdin".to_string()
}

fn default_format_on_save() -> bool {
    true
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct FormatterSettings {
    /// External command run with the document text on stdin; empty disables.
    #[serde(default = "default_formatter_command")]
    pub command: String,
    /// Space-separated arguments passed to the command.
    #[serde(default = "default_formatter_args")]
    pub args: String,
    /// Format the buffer immediately before saving.
    #[serde(default = "default_format_on_save")]
    pub format_on_save: bool,
}

impl Default for FormatterSettings {
    fn default() -> Self {
        Self {
            command: default_formatter_command(),
            args: default_formatter_args(),
            format_on_save: default_format_on_save(),
        }
    }
}

fn default_explorer_exclude() -> Vec<String> {
    vec![".git".to_string()]
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct ExplorerSettings {
    /// Gitignore-style patterns dropped from the file tree entirely, matched
    /// against the workspace root (`.git` reaches any depth, `/*.git` does not).
    #[serde(default = "default_explorer_exclude")]
    pub exclude: Vec<String>,
}

impl Default for ExplorerSettings {
    fn default() -> Self {
        Self {
            exclude: default_explorer_exclude(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
pub struct EditorSettings {
    #[serde(default)]
    pub theme: ThemeSettings,
    #[serde(default)]
    pub font: FontSettings,
    #[serde(default)]
    pub formatter: FormatterSettings,
    #[serde(default)]
    pub explorer: ExplorerSettings,
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
    fn partial_config_missing_keys_take_defaults() {
        let settings: EditorSettings = serde_json::from_str(r#"{"font":{"size":20.0}}"#).unwrap();
        assert_eq!(settings.font.family, "JetBrains Mono");
        assert_eq!(settings.font.size, 20.0);
        assert_eq!(settings.theme.choice, "auto");
        assert_eq!(settings.formatter.args, "--stdin");
        assert_eq!(settings.explorer.exclude, vec![".git".to_string()]);
    }

    #[test]
    fn an_explicit_empty_exclude_list_clears_the_default() {
        let settings: EditorSettings =
            serde_json::from_str(r#"{"explorer":{"exclude":[]}}"#).unwrap();
        assert!(settings.explorer.exclude.is_empty());
    }

    #[test]
    fn empty_config_resolves_to_defaults() {
        let settings: EditorSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, EditorSettings::default());
    }
}
