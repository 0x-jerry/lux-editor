//! Highlighting domain: the syntax service and its debounced refresh.

use super::App;
use crate::language::{
    HighlightSnapshot, HighlightThemeConfig, HighlightingService, LanguageKind,
};
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
        let choice = ThemeChoice::from_value(&self.settings.editor_config.settings.theme.choice);
        let theme_name = if choice == ThemeChoice::Auto {
            syntax_theme_for(ThemeChoice::Auto, self.chrome.last_system_theme)
        } else {
            syntax_theme_for(choice, None)
        };
        self.highlighting.service.set_theme(HighlightThemeConfig {
            theme_name: theme_name.to_string(),
            theme_path: self.settings.editor_config.settings.theme.theme_path.clone(),
        });
        let language = LanguageKind::from_path(self.buffer().path().map(|v| &**v));
        self.highlighting
            .service
            .request_parse(self.buffer().text().to_string(), language);
    }

    pub(super) fn highlight_snapshot(&self) -> &HighlightSnapshot {
        self.highlighting.service.snapshot()
    }
}