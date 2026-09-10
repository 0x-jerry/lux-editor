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
        // The palette goes in first: it carries the background the editor and
        // the tab strip are painted with, and that has to follow a theme change
        // even when there is no document worth re-parsing.
        let colors = self.syntax_colors();
        self.highlighting.service.set_syntax(colors);
        // While the configuration tab is active its own page is where the theme
        // gets changed, so the fallback document is the one whose snapshot the
        // chrome is still drawing from.
        let Some(target) = self.tabs.highlight_target() else {
            return;
        };
        let language = LanguageKind::from_path(target.buffer.path().map(|v| &**v));
        // Rope clone is O(1); the worker parses the shared text zero-copy.
        let text = target.buffer.text().clone();
        self.highlighting.service.request_parse(text, language);
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

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::document::DocumentBuffer;
    use crate::events::LoadResult;
    use crate::settings::Config;
    use crate::theme::{self, StartupFont, ThemeChoice};
    use eframe::egui;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    /// The tab strip and the editor background are painted from the highlight
    /// snapshot, which is built with the palette of the parse that produced it.
    /// Switching the theme happens on the configuration tab, so the refresh has
    /// to reach a document even though no text tab holds focus — otherwise the
    /// strip keeps the previous theme's background.
    #[test]
    fn theme_change_on_the_configuration_tab_repaints_the_editor_background() {
        let mut app = App::new(
            egui::Context::default(),
            StartupFont::spawn(String::new()),
            Config::default(),
        );

        let mut buffer = DocumentBuffer::new();
        buffer.set_path("/ws/a.rs");
        buffer.insert(0, "fn main() {}\n");
        app.tabs.apply_loaded(
            vec![(PathBuf::from("/ws/a.rs"), LoadResult::Loaded(buffer))],
            None,
        );
        app.tabs.open_configuration();
        assert!(app.tabs.active_is_configuration());

        app.chrome.runtime_theme = Some(ThemeChoice::Dark);
        {
            let mut cx = app.ctx();
            cx.refresh_language_intelligence();
        }
        let dark = theme::syntax_colors(ThemeChoice::Dark).background;
        assert_eq!(wait_for_background(&mut app, dark), Some(dark));

        // The user picks the light theme while the configuration page is up.
        app.settings.editor_config.settings.theme.choice = ThemeChoice::Light.value().to_string();
        app.chrome.runtime_theme = Some(ThemeChoice::Light);
        {
            let mut cx = app.ctx();
            assert!(cx.syntax_colors_changed());
            cx.refresh_language_intelligence();
        }

        let light = theme::syntax_colors(ThemeChoice::Light).background;
        assert_ne!(
            dark, light,
            "the two themes must differ for this to mean anything"
        );
        assert_eq!(wait_for_background(&mut app, light), Some(light));
    }

    /// The parse lands on the worker thread; poll the service the way the frame
    /// loop does until the snapshot catches up.
    fn wait_for_background(app: &mut App, expected: [u8; 4]) -> Option<[u8; 4]> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let background = app.highlighting.service.snapshot().background;
            if background == Some(expected) || Instant::now() >= deadline {
                return background;
            }
            app.highlighting.service.update();
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
