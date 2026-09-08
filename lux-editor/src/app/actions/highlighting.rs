//! Highlighting actions: the debounced language refresh and the palette probe
//! that keeps chrome-only restyles (font changes) from re-parsing the buffer.

use crate::app::Ctx;
use crate::highlighting::LanguageKind;
use crate::theme::{self, SyntaxColors, ThemeChoice};
use std::sync::Arc;

impl Ctx<'_> {
    pub(crate) fn schedule_language_refresh(&mut self) {
        self.highlighting.schedule_refresh();
    }

    pub(crate) fn flush_scheduled_language_refresh(&mut self) {
        if self.highlighting.take_due_refresh() {
            self.refresh_language_intelligence();
        }
    }

    pub(crate) fn refresh_language_intelligence(&mut self) {
        let colors = self.syntax_colors();
        let language = LanguageKind::from_path(self.documents.buffer().path().map(|v| &**v));
        self.highlighting.service.set_syntax(colors);
        // Rope clone is O(1); the worker parses the shared text zero-copy.
        self.highlighting
            .service
            .request_parse(self.documents.buffer().text().clone(), language);
    }

    /// The syntax palette the current config asks for.
    fn syntax_colors(&mut self) -> Arc<SyntaxColors> {
        // Before the first style pass the raw choice is all there is (`Auto` → dark).
        let choice = self.chrome.runtime_theme.unwrap_or_else(|| {
            ThemeChoice::from_value(&self.settings.editor_config.settings.theme.choice)
        });
        theme::syntax_colors(choice)
    }

    /// Whether the applied syntax palette drifted from the configured one; keeps
    /// chrome-only changes (fonts) from re-parsing the whole buffer.
    pub(crate) fn syntax_colors_changed(&mut self) -> bool {
        let colors = self.syntax_colors();
        !Arc::ptr_eq(self.highlighting.service.syntax(), &colors)
    }
}
