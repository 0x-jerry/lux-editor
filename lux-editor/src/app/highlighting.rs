//! Highlighting domain: the syntax service and its debounced refresh.

use super::App;
use crate::language::HighlightingService;
use crate::language::LanguageKind;
use crate::theme::{self, SyntaxColors, ThemeChoice};
use std::sync::Arc;
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
        self.highlighting.service.set_syntax(self.syntax_colors());
        let language = LanguageKind::from_path(self.buffer().path().map(|v| &**v));
        // Rope clone is O(1); the worker parses the shared text zero-copy.
        self.highlighting
            .service
            .request_parse(self.buffer().text().clone(), language);
    }

    /// The syntax palette the current config asks for.
    fn syntax_colors(&self) -> Arc<SyntaxColors> {
        // Before the first style pass the raw choice is all there is (`Auto` → dark).
        let choice = self.chrome.runtime_theme.unwrap_or_else(|| {
            ThemeChoice::from_value(&self.settings.editor_config.settings.theme.choice)
        });
        theme::syntax_colors(choice)
    }

    /// Whether the applied syntax palette drifted from the configured one; keeps
    /// chrome-only changes (fonts) from re-parsing the whole buffer.
    pub(super) fn syntax_colors_changed(&self) -> bool {
        !Arc::ptr_eq(self.highlighting.service.syntax(), &self.syntax_colors())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn service_round_trips_rope_snapshot() {
        let mut service = HighlightingService::new();
        service.request_parse(Rope::from_str("fn main() {}\n"), LanguageKind::Rust);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while service.snapshot().version == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
            service.update();
        }
        assert_eq!(service.snapshot().version, 1);
        assert!(!service.snapshot().line_tokens[0].is_empty());
    }
}
