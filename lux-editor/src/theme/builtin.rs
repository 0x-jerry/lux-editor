use std::sync::OnceLock;

use super::choice::ThemeChoice;
use super::file::ChromeColors;

const DARK_JSON: &str = include_str!("../../assets/themes/dark.json");
const LIGHT_JSON: &str = include_str!("../../assets/themes/light.json");

/// The embedded built-in theme for a resolved choice (`Auto` → dark).
pub fn builtin(choice: ThemeChoice) -> &'static ChromeColors {
    static DARK: OnceLock<ChromeColors> = OnceLock::new();
    static LIGHT: OnceLock<ChromeColors> = OnceLock::new();
    match choice {
        ThemeChoice::Light => {
            LIGHT.get_or_init(|| ChromeColors::parse(LIGHT_JSON).expect("builtin light theme"))
        }
        _ => DARK.get_or_init(|| ChromeColors::parse(DARK_JSON).expect("builtin dark theme")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_themes_parse_and_differ() {
        let dark = builtin(ThemeChoice::Dark);
        let light = builtin(ThemeChoice::Light);
        assert_ne!(dark.panel_bg, light.panel_bg);
        assert!(dark.panel_bg[0] < light.panel_bg[0]);
    }

    #[test]
    fn unknown_fields_in_theme_are_rejected() {
        let broken = DARK_JSON.replace("\"accent\":", "\"accentX\":");
        assert!(ChromeColors::parse(&broken).is_err());
    }
}
