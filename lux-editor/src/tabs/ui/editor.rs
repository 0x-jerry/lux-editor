use crate::chrome::ui::welcome::WelcomeView;
use crate::chrome::ui::{
    CaretInput, DocumentInput, FileBinaryInput, FileBinaryView, FileImageInput, FileImageView,
    FileMissingInput, FileMissingView, TabsInput, WorkspaceInput, WorkspaceStartInput,
    WorkspaceStartView, is_image_path,
};
use crate::component::Component;
use crate::document::ui::markdown::{MarkdownPreviewState, MarkdownView};
use crate::document::ui::{ScrollPane, ScrollSync, TextEditor, TextEditorState};
use crate::events::CustomEvent;
use crate::highlighting::snapshot_color;
use crate::settings::Config;
use crate::settings::configuration_view::{ConfigurationView, ConfigurationViewInput};
use crate::tabs::{TabStripInput, TabStripView};
use crate::theme::SyntaxColors;
use eframe::egui;
use std::sync::Arc;

/// The editor-area content dispatcher: shared tab strip plus whatever the
/// active tab holds (the configuration form or the text editor pages).
pub struct EditorView;

/// The shell-owned markdown preview session, borrowed for one frame: the
/// renderer state plus the scroll mirroring with the text editor.
pub struct MarkdownPreview<'a> {
    pub state: &'a mut MarkdownPreviewState,
    pub scroll: &'a mut ScrollSync,
    pub syntax: &'a Arc<SyntaxColors>,
}

pub struct EditorViewState<'a> {
    pub workspace: WorkspaceInput<'a>,
    pub tabs: TabsInput<'a>,
    pub document: DocumentInput<'a>,
    pub caret: CaretInput<'a>,
    /// The shell-owned configuration session; only used while the
    /// configuration tab is active.
    pub configuration: Option<&'a mut ConfigurationView>,
    pub editor_config: &'a Config,
    pub sidebar_visible: bool,
    /// `Some` while the markdown preview panel should render right of the
    /// text editor.
    pub markdown_preview: Option<MarkdownPreview<'a>>,
}

impl Component for EditorView {
    type Message = CustomEvent;
    type Input<'a> = EditorViewState<'a>;

    fn render(&mut self, ui: &mut egui::Ui, state: Self::Input<'_>) -> Vec<CustomEvent> {
        let EditorViewState {
            workspace,
            tabs,
            document,
            caret,
            configuration,
            editor_config,
            sidebar_visible,
            markdown_preview,
        } = state;
        let mut events = Vec::new();

        let editor_bg = snapshot_color(
            document.highlight_snapshot.background,
            ui.visuals().code_bg_color,
        );

        let nothing_open = document.buffer.path().is_none()
            && !document.dirty
            && document.buffer.text().len_chars() == 0
            && !workspace.restoring_session;

        if strip_visible(tabs.tabs.len(), workspace.path.is_some(), nothing_open) {
            let mut tabs_view = TabStripView;
            events.extend(
                tabs_view
                    .render(
                        ui,
                        TabStripInput {
                            tabs: tabs.tabs,
                            active_id: tabs.active_id,
                            background: editor_bg,
                        },
                    )
                    .into_iter()
                    .map(CustomEvent::Document),
            );
        }

        if tabs.active_is_configuration {
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

        if workspace.path.is_none() && document.buffer.path().is_none() {
            let mut welcome_view = WelcomeView;
            events.extend(welcome_view.render(ui, editor_config));
            return events;
        }

        if let (Some(path), true) = (workspace.path, nothing_open) {
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
        if document.missing {
            if let Some(path) = document.buffer.path() {
                let mut missing_view = FileMissingView;
                missing_view.render(
                    ui,
                    FileMissingInput {
                        path,
                        loader_error: document.status,
                    },
                );
            }
            return events;
        }

        // A binary file is not editable, so a guide page replaces the text
        // area; the tab is a real file though, never struck through. Images are
        // the one binary kind with something to show: a viewer replaces the
        // guide page.
        if document.binary {
            if let Some(path) = document.buffer.path() {
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
            let markdown = document.buffer.text().to_string();
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
                        MarkdownView::show(ui, preview.state, &markdown, preview.syntax);
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
                        buffer: document.buffer,
                        highlight_snapshot: document.highlight_snapshot,
                        editor_config,
                        carets: caret.carets,
                        selection_ranges: caret.selection_ranges,
                        active_caret_index: caret.active_index,
                        caret_visible: caret.visible,
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
