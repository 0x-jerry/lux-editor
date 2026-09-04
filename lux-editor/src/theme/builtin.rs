use std::sync::{Arc, OnceLock};

use super::choice::ThemeChoice;
use super::file::{SyntaxColors, ThemeFile};

const DARK_JSON: &str = include_str!("../../assets/themes/dark.json");
const LIGHT_JSON: &str = include_str!("../../assets/themes/light.json");

/// The embedded built-in theme for a resolved choice (`Auto` → dark).
pub fn builtin(choice: ThemeChoice) -> &'static ThemeFile {
    static DARK: OnceLock<ThemeFile> = OnceLock::new();
    static LIGHT: OnceLock<ThemeFile> = OnceLock::new();
    match choice {
        ThemeChoice::Light => {
            LIGHT.get_or_init(|| ThemeFile::parse(LIGHT_JSON).expect("builtin light theme"))
        }
        _ => DARK.get_or_init(|| ThemeFile::parse(DARK_JSON).expect("builtin dark theme")),
    }
}

/// Shared handle to a built-in syntax palette; `Arc::ptr_eq` tells whether
/// the palette actually changed, so chrome-only edits skip a re-parse.
pub fn syntax_colors(choice: ThemeChoice) -> Arc<SyntaxColors> {
    Arc::clone(&builtin(choice).syntax)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_themes_parse_and_differ() {
        let dark = builtin(ThemeChoice::Dark);
        let light = builtin(ThemeChoice::Light);
        assert_ne!(dark.colors.panel_bg, light.colors.panel_bg);
        assert!(dark.colors.panel_bg.r() < light.colors.panel_bg.r());
        assert_ne!(dark.syntax.background, light.syntax.background);
    }

    #[test]
    fn syntax_palette_handles_are_stable() {
        assert!(Arc::ptr_eq(
            &syntax_colors(ThemeChoice::Dark),
            &syntax_colors(ThemeChoice::Dark)
        ));
        assert!(!Arc::ptr_eq(
            &syntax_colors(ThemeChoice::Dark),
            &syntax_colors(ThemeChoice::Light)
        ));
    }

    #[test]
    fn unknown_fields_in_theme_are_rejected() {
        let broken = DARK_JSON.replace("\"accent\":", "\"accentX\":");
        assert!(ThemeFile::parse(&broken).is_err());
    }
}
