use crate::chrome::ui::welcome::WelcomeView;
use crate::chrome::ui::{
    FileBinaryInput, FileBinaryView, FileImageInput, FileImageView, FileMissingInput,
    FileMissingView, WorkspaceStartInput, WorkspaceStartView, is_image_path,
};
use crate::component::Component;
use crate::document::DocumentBuffer;
use crate::document::ui::{ScrollPane, ScrollSync, TextEditor, TextEditorState};
use crate::events::CustomEvent;
use crate::highlighting::HighlightSnapshot;
use crate::highlighting::snapshot_color;
use crate::settings::Config;
use crate::settings::configuration_view::{ConfigurationView, ConfigurationViewInput};
use crate::tabs::{TabMeta, TabStripInput, TabStripView};
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::ops::Range;
use std::path::PathBuf;

/// The editor-area content dispatcher: shared tab strip plus whatever the
/// active tab holds (the configuration form or the text editor pages).
pub struct EditorView;

/// The shell-owned markdown preview session, borrowed for one frame: the
/// renderer cache plus the scroll mirroring with the text editor.
pub struct MarkdownPreview<'a> {
    pub cache: &'a mut CommonMarkCache,
    pub scroll: &'a mut ScrollSync,
}

pub struct EditorViewState<'a> {
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a DocumentBuffer,
    pub tabs: &'a [TabMeta],
    pub active_tab_id: u64,
    pub active_is_configuration: bool,
    /// The shell-owned configuration session; only used while the
    /// configuration tab is active.
    pub configuration: Option<&'a mut ConfigurationView>,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    /// All cursor positions as 1-based (line, column).
    pub carets: &'a [(usize, usize)],
    pub selection_ranges: &'a [Range<usize>],
    pub active_caret_index: usize,
    pub caret_visible: bool,
    pub sidebar_visible: bool,
    pub document_dirty: bool,
    pub document_status: Option<&'a str>,
    pub document_missing: bool,
    pub document_binary: bool,
    pub restoring_session: bool,
    /// `Some` while the markdown preview panel should render right of the
    /// text editor.
    pub markdown_preview: Option<MarkdownPreview<'a>>,
}

impl Component for EditorView {
    type Message = CustomEvent;
    type Input<'a> = EditorViewState<'a>;

    fn render(&mut self, ui: &mut egui::Ui, state: Self::Input<'_>) -> Vec<CustomEvent> {
        let EditorViewState {
            workspace_path,
            buffer,
            tabs,
            active_tab_id,
            active_is_configuration,
            configuration,
            highlight_snapshot,
            editor_config,
            carets,
            selection_ranges,
            active_caret_index,
            caret_visible,
            sidebar_visible,
            document_dirty,
            document_status,
            document_missing,
            document_binary,
            restoring_session,
            markdown_preview,
        } = state;
        let mut events = Vec::new();

        let editor_bg = snapshot_color(highlight_snapshot.background, ui.visuals().code_bg_color);

        let nothing_open = buffer.path().is_none()
            && !document_dirty
            && buffer.text().len_chars() == 0
            && !restoring_session;

        if strip_visible(tabs.len(), workspace_path.is_some(), nothing_open) {
            let mut tabs_view = TabStripView;
            events.extend(
                tabs_view
                    .render(
                        ui,
                        TabStripInput {
                            tabs,
                            active_id: active_tab_id,
                            background: editor_bg,
                        },
                    )
                    .into_iter()
                    .map(CustomEvent::Document),
            );
        }

        if active_is_configuration {
            if let Some(configuration) = configuration {
                events.extend(
                    configuration
                        .render(ui, ConfigurationViewInput { editor_config })
                        .into_iter()
                        .map(CustomEvent::Configuration),
                );
            }
            return events;
        }

        if workspace_path.is_none() && buffer.path().is_none() {
            let mut welcome_view = WelcomeView;
            events.extend(welcome_view.render(ui, editor_config));
            return events;
        }

        if let (Some(path), true) = (workspace_path, nothing_open) {
            let mut start_view = WorkspaceStartView;
            events.extend(start_view.render(
                ui,
                WorkspaceStartInput {
                    workspace_path: path,
                    sidebar_visible,
                },
            ));
            return events;
        }

        // A vanished file is not editable, so the page replaces the text area
        // and leaves the strip reachable for the other tabs.
        if document_missing {
            if let Some(path) = buffer.path() {
                let mut missing_view = FileMissingView;
                missing_view.render(
                    ui,
                    FileMissingInput {
                        path,
                        loader_error: document_status,
                    },
                );
            }
            return events;
        }

        // A binary file is not editable, so a guide page replaces the text
        // area; the tab is a real file though, never struck through. Images are
        // the one binary kind with something to show: a viewer replaces the
        // guide page.
        if document_binary {
            if let Some(path) = buffer.path() {
                if is_image_path(path) {
                    let mut image_view = FileImageView;
                    image_view.render(ui, FileImageInput { path });
                } else {
                    let mut binary_view = FileBinaryView;
                    binary_view.render(ui, FileBinaryInput { path });
                }
            }
            return events;
        }

        // Whichever pane the pointer rests on drives the shared scroll; the
        // other is pinned to its fraction. `None` (no preview) leaves the text
        // editor scrolling alone.
        let mut scroll_sync: Option<&mut ScrollSync> = None;
        if let Some(preview) = markdown_preview {
            // Rope -> String every frame keeps the preview live while typing.
            // Ceiling: huge documents pay a per-frame copy + re-parse; cache
            // by `edit_generation` if that ever shows up in a profile.
            let markdown = buffer.text().to_string();
            // Never wider than 80% of the tab content view, so the editor
            // always keeps a usable column beside the preview.
            let max_width: f32 = ui.available_width() * 0.8;
            preview.scroll.update_driver(ui);
            let follow_offset = preview.scroll.follow_offset(ScrollPane::Preview);
            let panel = egui::Panel::right("markdown_preview")
                .resizable(true)
                .default_size(420.0)
                .size_range(max_width.min(200.0)..=max_width)
                .frame(egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::same(0)))
                .show(ui, |ui| {
                    // Padding lives on the content, not the panel frame: a frame
                    // margin shrinks the viewport and so changes how far the
                    // pane can scroll, which the editor mirror would follow.
                    let mut scroll_area = egui::ScrollArea::vertical()
                        .content_margin(12)
                        .auto_shrink([false, false]);
                    if let Some(offset) = follow_offset {
                        scroll_area = scroll_area.vertical_scroll_offset(offset);
                    }
                    scroll_area.show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        CommonMarkViewer::new().show(ui, preview.cache, &markdown);
                    })
                });
            let scroll = &panel.inner;
            preview.scroll.report(
                ScrollPane::Preview,
                scroll.state.offset.y,
                scroll.inner_rect,
                scroll.content_size,
            );
            // The text editor renders next, on the other side of the mirror.
            scroll_sync = Some(preview.scroll);
        }

        let mut text_editor = TextEditor;
        events.extend(
            text_editor
                .render(
                    ui,
                    TextEditorState {
                        buffer,
                        highlight_snapshot,
                        editor_config,
                        carets,
                        selection_ranges,
                        active_caret_index,
                        caret_visible,
                        scroll_sync,
                    },
                )
                .into_iter()
                .map(CustomEvent::Editing),
        );
        events
    }
}

/// Whether the tab strip earns its row. Past one tab it always does: the strip
/// is the only way to reach a tab (no keybinding switches). With a lone tab
/// there is nothing to switch to, so only a workspace holding something open
/// keeps it around.
fn strip_visible(tab_count: usize, has_workspace: bool, nothing_open: bool) -> bool {
    tab_count > 1 || (has_workspace && !nothing_open)
}

#[cfg(test)]
mod tests {
    use super::strip_visible;

    #[test]
    fn strip_follows_tab_count_and_workspace() {
        // Single file open, no workspace: content only.
        assert!(!strip_visible(1, false, false));
        // Nothing open at all (welcome page).
        assert!(!strip_visible(1, false, true));
        // Workspace on its start page: no tab to show yet.
        assert!(!strip_visible(1, true, true));
        // Workspace with a file open.
        assert!(strip_visible(1, true, false));
        // Several tabs stay reachable, workspace or not.
        assert!(strip_visible(2, false, false));
        assert!(strip_visible(2, true, true));
    }
}
