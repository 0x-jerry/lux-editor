//! Opening and closing tabs: files from disk, untitled buffers, the settings
//! page, and the unsaved-changes gate in front of a close.

use super::Root;
use crate::gpui_ui::document::OpenDocument;
use crate::gpui_ui::language::language_for;
use crate::gpui_ui::markdown;
use crate::gpui_ui::tabs::{Tab, TabContent};
use crate::gpui_ui::views::{self, Content, Notice};
use gpui_kit::component::input::{EditorState, InputEvent};
use gpui_kit::*;
use std::path::PathBuf;

/// The answer to "this tab has unsaved changes".
enum Decision {
    Save,
    Discard,
    Cancel,
}

impl Root {
    /// Open whatever the command line named: a folder becomes the workspace, a
    /// file is loaded as a tab. Anything else is ignored.
    pub(super) fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.canonicalize().unwrap_or(path);
        if path.is_dir() {
            self.open_folder(path, window, cx);
        } else if path.is_file() {
            self.open_file(path, window, cx);
        } else {
            log::warn!("ignoring command-line path {}", path.display());
        }
    }

    pub(super) fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.state.tabs.index_of(&path) {
            self.activate(index, window, cx);
            return;
        }

        let (content, list_as_recent) = match views::classify(&path) {
            Content::Text(text) => (
                self.new_text_tab(text, Some(path.clone()), window, cx),
                true,
            ),
            Content::Image => (TabContent::Image(path.clone()), true),
            Content::Binary => (
                TabContent::Notice {
                    path: path.clone(),
                    kind: Notice::Binary,
                },
                false,
            ),
            Content::Missing => (
                TabContent::Notice {
                    path: path.clone(),
                    kind: Notice::Missing,
                },
                false,
            ),
        };

        self.push_content(content);
        // In-workspace files belong to the workspace session; recents are for
        // the files opened from outside it.
        let outside_workspace = self
            .state
            .workspace
            .path
            .as_deref()
            .is_none_or(|root| !path.starts_with(root));
        if list_as_recent && outside_workspace {
            self.state.config.add_recent(path, false);
        }
        self.sync_window_title(window);
        cx.notify();
    }

    /// Create the window's first tab: an empty untitled document.
    pub(super) fn new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let content = self.new_text_tab(String::new(), None, window, cx);
        self.push_content(content);
        self.sync_window_title(window);
        cx.notify();
    }

    /// Load `text` into a new editor and wrap it as a text tab body, wiring the
    /// editor's change events to the tab's dirty flag.
    fn new_text_tab(
        &mut self,
        text: String,
        path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> TabContent {
        let language = path.as_deref().and_then(language_for);
        let editor = cx.new(|cx| {
            let mut editor = EditorState::new(window, cx).line_number(true);
            if let Some(language) = language {
                editor = editor.language(language);
            }
            editor.default_value(text)
        });

        let id = self.state.tabs.next_id();
        let changes = cx.subscribe(&editor, move |root, _editor, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                root.state.tabs.mark_dirty(id);
                cx.notify();
            }
        });
        TabContent::Text(OpenDocument {
            editor,
            path,
            dirty: false,
            preview: false,
            _changes: changes,
        })
    }

    /// Add a tab and make it the active one.
    fn push_content(&mut self, content: TabContent) {
        let id = self.state.tabs.next_id();
        self.state.tabs.push(Tab { id, content });
    }

    /// Show the settings tab, creating it the first time it is asked for.
    pub(super) fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.state.tabs.settings_index() {
            self.activate(index, window, cx);
            return;
        }
        let settings = self.state.config.settings.clone();
        self.settings_view
            .update(cx, |view, cx| view.sync(&settings, cx));
        self.push_content(TabContent::Settings);
        self.sync_window_title(window);
        cx.notify();
    }

    pub(super) fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.state.tabs.active = index;
        self.sync_window_title(window);
        cx.notify();
    }

    /// Flip the active document between its source and its rendered markdown.
    pub(super) fn toggle_preview(&mut self, cx: &mut Context<Self>) {
        let Some(doc) = self.state.tabs.active_doc_mut() else {
            return;
        };
        let Some(path) = doc.path.clone() else {
            return;
        };
        if !markdown::is_markdown(&path) {
            log::debug!("no preview for {}", path.display());
            return;
        }
        doc.preview = !doc.preview;
        cx.notify();
    }

    pub(super) fn open_file_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.open_file(path, window, cx);
        }
    }

    pub(super) fn open_folder_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.open_folder(path, window, cx);
        }
    }

    /// Close the active tab, prompting first when it has unsaved changes.
    pub(super) fn close_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.state.tabs.active_index() {
            self.close_tab(index, window, cx);
        }
    }

    pub(super) fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if !self.confirm_tab(index, window, cx) {
            return;
        }
        self.state.tabs.close(index);
        self.sync_window_title(window);
        cx.notify();
    }

    /// Ask about tab `index` when it has unsaved changes, and for a `Save`
    /// answer write it. Returns whether the tab may now be discarded.
    fn confirm_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.state.tabs.is_dirty(index) {
            return true;
        }
        match self.ask_about(index) {
            Decision::Save => self.save_index(index, window, cx),
            Decision::Discard => true,
            Decision::Cancel => false,
        }
    }

    /// Walk the unsaved documents, asking about each. Returns whether every one
    /// of them may be discarded, which is what a close or a quit needs.
    pub(super) fn confirm_all_dirty(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        for index in 0..self.state.tabs.len() {
            if !self.confirm_tab(index, window, cx) {
                return false;
            }
        }
        true
    }

    /// Put the unsaved-changes question to the user for tab `index`.
    fn ask_about(&self, index: usize) -> Decision {
        let name = self
            .state
            .tabs
            .tab(index)
            .map(Tab::title)
            .unwrap_or_default();
        let answer = rfd::MessageDialog::new()
            .set_title("Unsaved changes")
            .set_description(format!("Save changes to {name}?"))
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show();
        match answer {
            rfd::MessageDialogResult::Yes => Decision::Save,
            rfd::MessageDialogResult::No => Decision::Discard,
            _ => Decision::Cancel,
        }
    }
}
