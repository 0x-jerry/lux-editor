//! Writing a buffer back to disk: Save and Save As, and the configured
//! formatter that rewrites the text on its way out.

use super::Root;
use gpui_kit::*;

impl Root {
    /// Save the active document, prompting for a path when it has none.
    pub(super) fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.state.tabs.active_index() else {
            return;
        };
        self.save_index(index, window, cx);
    }

    /// Save tab `index`; returns whether the write landed. A tab without a path
    /// goes through the Save As dialog, and a configured format-on-save
    /// formatter rewrites the text before it is written.
    pub(super) fn save_index(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(path) = self.state.tabs.get(index).and_then(|doc| doc.path.clone()) else {
            return self.save_index_as(index, window, cx);
        };
        let Some(text) = self.state.tabs.get(index).map(|doc| doc.text(cx)) else {
            return false;
        };
        let text = match self.format_on_save(index, &text, window, cx) {
            Some(formatted) => formatted,
            None => text,
        };
        match crate::atomic::write(&path, text.as_bytes()) {
            Ok(()) => {
                if let Some(doc) = self.state.tabs.get_mut(index) {
                    doc.dirty = false;
                }
                self.state.config.add_recent(path, false);
                self.sync_window_title(window);
                cx.notify();
                true
            }
            Err(err) => {
                log::error!("failed to save {}: {err}", path.display());
                false
            }
        }
    }

    /// The formatted text for tab `index`, or `None` to write `text` unchanged.
    /// The buffer adopts the formatted text, so what is on disk and on screen
    /// stay the same; a formatter failure only logs, and the save goes ahead.
    fn format_on_save(
        &mut self,
        index: usize,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let formatter = self.state.config.settings.formatter.clone();
        if !formatter.format_on_save || formatter.command.trim().is_empty() {
            return None;
        }
        let formatted = match crate::formatter::run_formatter(
            &formatter.command,
            &formatter.args,
            text,
        ) {
            Ok(formatted) => formatted,
            Err(err) => {
                log::warn!("formatter failed, saving unformatted: {err}");
                return None;
            }
        };
        if formatted == text {
            return None;
        }
        self.replace_text(index, &formatted, true, window, cx);
        Some(formatted)
    }

    /// Format the active document in place, leaving it unsaved.
    pub(super) fn format_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let formatter = self.state.config.settings.formatter.clone();
        if formatter.command.trim().is_empty() {
            log::debug!("no formatter configured");
            return;
        }
        let Some(doc) = self.state.tabs.active_doc() else {
            return;
        };
        let text = doc.text(cx);
        match crate::formatter::run_formatter(&formatter.command, &formatter.args, &text) {
            Ok(formatted) if formatted != text => {
                let Some(index) = self.state.tabs.active_index() else {
                    return;
                };
                self.replace_text(index, &formatted, false, window, cx);
            }
            Ok(_) => log::debug!("already formatted"),
            Err(err) => log::warn!("formatter failed: {err}"),
        }
    }

    /// Replace a document's whole text; `saved` marks it as matching the disk,
    /// because setting the value does not emit an edit event.
    fn replace_text(
        &mut self,
        index: usize,
        text: &str,
        saved: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(doc) = self.state.tabs.get_mut(index) else {
            return;
        };
        doc.editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        doc.dirty = !saved;
        self.sync_window_title(window);
        cx.notify();
    }

    /// Save As: ask for a destination, then write there and adopt the path.
    fn save_index_as(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(doc) = self.state.tabs.get(index) else {
            return false;
        };
        let text = doc.text(cx);
        let current = doc.path.clone();
        let mut dialog = rfd::FileDialog::new();
        if let Some(dir) = current.as_ref().and_then(|path| path.parent()) {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.save_file() else {
            return false;
        };
        let text = match self.format_on_save(index, &text, window, cx) {
            Some(formatted) => formatted,
            None => text,
        };
        match crate::atomic::write(&path, text.as_bytes()) {
            Ok(()) => {
                if let Some(doc) = self.state.tabs.get_mut(index) {
                    doc.set_path(path.clone());
                }
                self.state.config.add_recent(path, false);
                self.sync_window_title(window);
                cx.notify();
                true
            }
            Err(err) => {
                log::error!("failed to save {}: {err}", path.display());
                false
            }
        }
    }

    pub(super) fn save_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.state.tabs.active_index() {
            self.save_index_as(index, window, cx);
        }
    }
}
