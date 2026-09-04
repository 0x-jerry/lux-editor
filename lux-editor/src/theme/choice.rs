use eframe::egui;

/// User-facing theme choice, persisted in config as a string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeChoice {
    Auto,
    Dark,
    Light,
}

impl ThemeChoice {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }

    pub fn value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn from_value(value: &str) -> Self {
        match value {
            "dark" => Self::Dark,
            "light" => Self::Light,
            _ => Self::Auto,
        }
    }
}

/// Collapse `Auto` against the system theme; never returns [`ThemeChoice::Auto`].
pub fn resolve(choice: ThemeChoice, system: Option<egui::Theme>) -> ThemeChoice {
    match choice {
        ThemeChoice::Dark => ThemeChoice::Dark,
        ThemeChoice::Light => ThemeChoice::Light,
        ThemeChoice::Auto => match system {
            Some(egui::Theme::Light) => ThemeChoice::Light,
            _ => ThemeChoice::Dark,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choice_round_trips_through_value() {
        for choice in [ThemeChoice::Auto, ThemeChoice::Dark, ThemeChoice::Light] {
            assert_eq!(ThemeChoice::from_value(choice.value()), choice);
        }
        assert_eq!(ThemeChoice::from_value("bogus"), ThemeChoice::Auto);
    }

    #[test]
    fn resolve_never_returns_auto() {
        for choice in [ThemeChoice::Auto, ThemeChoice::Dark, ThemeChoice::Light] {
            for system in [None, Some(egui::Theme::Dark), Some(egui::Theme::Light)] {
                assert_ne!(resolve(choice, system), ThemeChoice::Auto);
            }
        }
    }

    #[test]
    fn auto_resolves_against_system_theme() {
        assert_eq!(
            resolve(ThemeChoice::Auto, Some(egui::Theme::Light)),
            ThemeChoice::Light
        );
        assert_eq!(
            resolve(ThemeChoice::Auto, Some(egui::Theme::Dark)),
            ThemeChoice::Dark
        );
        assert_eq!(resolve(ThemeChoice::Auto, None), ThemeChoice::Dark);
    }
}
