//! Highlighting domain: the syntax service and its debounced refresh.

use super::App;
use crate::language::{HighlightSnapshot, HighlightThemeConfig, HighlightingService, LanguageKind};
use crate::ui::theme::{ThemeChoice, syntax_theme_for};
use std::time::{Duration, Instant};

pub(crate) struct Highlighting {
    pub(crate) service: HighlightingService,
    pub(crate) dirty: bool,
    pub(crate) deadline: Option<Instant>,
}

impl Default for Highlighting {
    fn default() -> Self {
        Self {
            service: HighlightingService::new(),
            dirty: false,
            deadline: None,
        }
    }
}

impl App {
    const HIGHLIGHT_DEBOUNCE: Duration = Duration::from_millis(60);

    pub(super) fn schedule_language_refresh(&mut self) {
        self.highlighting.dirty = true;
        self.highlighting.deadline = Some(Instant::now() + Self::HIGHLIGHT_DEBOUNCE);
    }

    pub(super) fn flush_scheduled_language_refresh(&mut self) {
        if self.highlighting.dirty
            && self
                .highlighting
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.highlighting.dirty = false;
            self.highlighting.deadline = None;
            self.refresh_language_intelligence();
        }
    }

    pub(super) fn refresh_language_intelligence(&mut self) {
        let config = self.syntax_theme_config();
        self.highlighting.service.set_theme(config);
        let language = LanguageKind::from_path(self.buffer().path().map(|v| &**v));
        self.highlighting
            .service
            .request_parse(self.buffer().text().to_string(), language);
    }

    /// The syntax theme the current config asks for.
    fn syntax_theme_config(&self) -> HighlightThemeConfig {
        // Before the first style pass the raw choice is all there is (`Auto` → dark).
        let choice = self.chrome.runtime_theme.unwrap_or_else(|| {
            ThemeChoice::from_value(&self.settings.editor_config.settings.theme.choice)
        });
        HighlightThemeConfig {
            theme_name: syntax_theme_for(choice, None).to_string(),
            theme_path: self
                .settings
                .editor_config
                .settings
                .theme
                .theme_path
                .clone(),
        }
    }

    /// Whether the applied syntax theme drifted from the configured one; keeps
    /// chrome-only changes (fonts) from re-parsing the whole buffer.
    pub(super) fn syntax_theme_changed(&self) -> bool {
        *self.highlighting.service.theme() != self.syntax_theme_config()
    }

    pub(super) fn highlight_snapshot(&self) -> &HighlightSnapshot {
        self.highlighting.service.snapshot()
    }
}
