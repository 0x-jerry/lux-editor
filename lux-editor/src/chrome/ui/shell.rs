//! The app shell component: chrome (title/status bars), sidebar and the
//! active view. Owns the shell chrome state (sidebar, configuration session)
//! and its child components.

use super::frame_input::{CaretInput, DocumentInput, SidebarInput, TabsInput, WorkspaceInput};
use super::widgets::{StatusBar, StatusBarData, TitleBar, TitleBarData, window_resize_handle};
use crate::component::Component;
use crate::document::ui::ScrollSync;
use crate::document::ui::markdown::MarkdownPreviewState;
use crate::events::CustomEvent;
use crate::highlighting::snapshot_color;
use crate::settings::configuration_view::ConfigurationView;
use crate::settings::{Config, EditorSettings};
use crate::tabs::{EditorView, EditorViewState, MarkdownPreview};
use crate::theme::SyntaxColors;
use crate::workspace::file_tree_panel::{FileTreePanel, FileTreePanelInput};
use eframe::egui;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

/// The document-model snapshot the shell renders from each frame.
pub struct ShellInput<'a> {
    pub sidebar: SidebarInput<'a>,
    pub workspace: WorkspaceInput<'a>,
    pub tabs: TabsInput<'a>,
    pub document: DocumentInput<'a>,
    pub caret: CaretInput<'a>,
    pub editor_config: &'a Config,
    /// The palette the editor's highlight snapshot was built with, so preview
    /// fences match it exactly.
    pub syntax: &'a Arc<SyntaxColors>,
}

/// The app shell: chrome (title/status bars), sidebar and the active view.
/// Owns the shell chrome state and its child components; the configuration
/// session exists only while a configuration tab is open.
pub struct Shell {
    // Chrome bars.
    title_bar: TitleBar,
    status_bar: StatusBar,

    // Sidebar.
    sidebar_visible: bool,
    file_tree_panel: FileTreePanel,

    // Markdown preview.
    markdown_preview_visible: bool,
    /// Preview renderer state (tree-sitter fence cache, link/scroll cache).
    markdown: MarkdownPreviewState,
    /// Scroll mirroring between the text editor and the markdown preview.
    markdown_scroll: ScrollSync,

    // Central views.
    editor_view: EditorView,
    pub(crate) configuration_view: Option<ConfigurationView>,
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            title_bar: TitleBar,
            status_bar: StatusBar,
            sidebar_visible: true,
            file_tree_panel: FileTreePanel::default(),
            markdown_preview_visible: false,
            markdown: MarkdownPreviewState::default(),
            markdown_scroll: ScrollSync::default(),
            editor_view: EditorView,
            configuration_view: None,
        }
    }
}

impl Shell {
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    pub fn toggle_markdown_preview(&mut self) {
        self.markdown_preview_visible = !self.markdown_preview_visible;
        if self.markdown_preview_visible {
            // Mirror from where the editor is now, not from the fraction a
            // closed preview left behind.
            self.markdown_scroll = ScrollSync::default();
        }
    }

    /// Seed the tree's expanded folders (workspace restore / first open).
    pub fn set_file_tree_expanded(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        self.file_tree_panel.set_expanded(paths);
    }

    pub fn file_tree_expanded(&self) -> &HashSet<PathBuf> {
        self.file_tree_panel.expanded()
    }

    pub fn sync_config_draft(&mut self, settings: &EditorSettings) {
        if let Some(view) = self.configuration_view.as_mut() {
            view.sync_draft(settings);
        }
    }
}

impl Component for Shell {
    type Message = CustomEvent;
    type Input<'a> = ShellInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, state: Self::Input<'_>) -> Vec<CustomEvent> {
        let ShellInput {
            sidebar,
            workspace,
            tabs,
            document,
            caret,
            editor_config,
            syntax,
        } = state;

        let mut events = Vec::new();

        self.title_bar.render(ui, TitleBarData { app_title: "Lux" });

        let selection_len: usize = caret
            .selection_ranges
            .iter()
            .map(|range| range.end - range.start)
            .sum();
        let (caret_line, caret_column) = caret
            .carets
            .get(caret.active_index)
            .copied()
            .unwrap_or((1, 1));
        // The image view publishes its geometry each frame; the status bar
        // reads last frame's, close enough for a readout.
        let image_status = if document.binary {
            document
                .buffer
                .path()
                .filter(|path| super::is_image_path(path))
                .and_then(|path| {
                    let id =
                        super::file_image::ImageStatus::data_id(&super::file_image::file_uri(path));
                    ui.data(|data| data.get_temp(id))
                })
        } else {
            None
        };
        events.extend(
            self.status_bar.render(
                ui,
                StatusBarData {
                    // Without a workspace there is no file tree, so the toggle
                    // would be dead chrome.
                    sidebar: sidebar.file_tree.is_some().then_some(self.sidebar_visible),
                    cursor: if tabs.active_is_configuration || image_status.is_some() {
                        None
                    } else {
                        Some((caret_line, caret_column, selection_len))
                    },
                    markdown_preview: tabs
                        .active_is_markdown
                        .then_some(self.markdown_preview_visible),
                    file_size: document.file_size,
                    image_status,
                },
            ),
        );

        if self.sidebar_visible
            && let Some(tree) = sidebar.file_tree
        {
            events.extend(self.file_tree_panel.render(
                ui,
                FileTreePanelInput {
                    tree,
                    // Only a file tab's path belongs in the tree; the
                    // configuration tab (or any non-file tab) selects nothing.
                    active_file_path: if tabs.active_is_configuration {
                        None
                    } else {
                        document.buffer.path().map(|path| path.as_path())
                    },
                },
            ));
        }

        // Markdown preview: the tab content view renders it right of the
        // editor, only while the active tab is an editable markdown document.
        let markdown_preview = (tabs.active_is_markdown
            && self.markdown_preview_visible
            && !document.missing
            && !document.binary)
            .then_some(MarkdownPreview {
                state: &mut self.markdown,
                scroll: &mut self.markdown_scroll,
                syntax,
            });

        let central_fill = if tabs.active_is_configuration {
            ui.visuals().panel_fill
        } else {
            snapshot_color(
                document.highlight_snapshot.background,
                ui.visuals().code_bg_color,
            )
        };
        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(ui.style())
                    .fill(central_fill)
                    .inner_margin(0),
            )
            .show(ui, |ui| {
                events.extend(self.editor_view.render(
                    ui,
                    EditorViewState {
                        workspace,
                        tabs,
                        document,
                        caret,
                        configuration: self.configuration_view.as_mut(),
                        editor_config,
                        sidebar_visible: self.sidebar_visible,
                        markdown_preview,
                    },
                ));
            });

        // Bottom-right resize grip for frameless window builds (no-op on macOS).
        egui::Area::new(egui::Id::new("window_resize_handle"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-3.0, -3.0))
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                window_resize_handle(ui);
            });

        events
    }
}
