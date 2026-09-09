//! The app shell component: chrome (title/status bars), sidebar and the
//! active view. Owns the shell chrome state (sidebar, configuration session)
//! and its child components.

use super::widgets::{
    StatusBar, StatusBarData, TitleBar, TitleBarData, window_resize_handle,
};
use crate::component::Component;
use crate::document::DocumentBuffer;
use crate::tabs::{EditorView, EditorViewState};
use crate::events::CustomEvent;
use crate::highlighting::HighlightSnapshot;
use crate::highlighting::snapshot_color;
use crate::settings::configuration_view::ConfigurationView;
use crate::settings::{Config, EditorSettings};
use crate::workspace::FileTree;
use crate::workspace::file_tree_panel::{FileTreePanel, FileTreePanelInput};
use eframe::egui;
use std::collections::HashSet;
use std::ops::Range;
use std::path::PathBuf;

/// The document-model snapshot the shell renders from each frame.
pub struct ShellInput<'a> {
    pub file_tree: Option<&'a mut FileTree>,
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a DocumentBuffer,
    pub tabs: &'a [crate::tabs::TabMeta],
    pub active_tab_id: u64,
    pub active_is_configuration: bool,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    pub document_status: Option<&'a str>,
    pub restoring_session: bool,
    /// All cursor positions as 1-based (line, column).
    pub carets: Vec<(usize, usize)>,
    pub selection_ranges: Vec<Range<usize>>,
    pub active_caret_index: usize,
    pub caret_visible: bool,
    pub document_dirty: bool,
    pub document_missing: bool,
    pub document_binary: bool,
}

/// The app shell: chrome (title/status bars), sidebar and the active view.
/// Owns the shell chrome state and its child components; the configuration
/// session exists only while a configuration tab is open.
pub struct Shell {
    sidebar_visible: bool,
    title_bar: TitleBar,
    status_bar: StatusBar,
    file_tree_panel: FileTreePanel,
    editor_view: EditorView,
    pub(crate) configuration_view: Option<ConfigurationView>,
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            sidebar_visible: true,
            title_bar: TitleBar,
            status_bar: StatusBar,
            file_tree_panel: FileTreePanel::default(),
            editor_view: EditorView,
            configuration_view: None,
        }
    }
}

impl Shell {
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
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
            file_tree,
            workspace_path,
            buffer,
            tabs,
            active_tab_id,
            active_is_configuration,
            highlight_snapshot,
            editor_config,
            document_status,
            restoring_session,
            carets,
            selection_ranges,
            active_caret_index,
            caret_visible,
            document_dirty,
            document_missing,
            document_binary,
        } = state;

        let mut events = Vec::new();

        self.title_bar.render(ui, TitleBarData { app_title: "Lux" });

        let selection_len: usize = selection_ranges
            .iter()
            .map(|range| range.end - range.start)
            .sum();
        let (caret_line, caret_column) = carets.get(active_caret_index).copied().unwrap_or((1, 1));
        events.extend(self.status_bar.render(
            ui,
            StatusBarData {
                sidebar_active: self.sidebar_visible,
                cursor: if active_is_configuration {
                    None
                } else {
                    Some((caret_line, caret_column, selection_len))
                },
            },
        ));

        if self.sidebar_visible
            && let Some(tree) = file_tree
        {
            events.extend(self.file_tree_panel.render(
                ui,
                FileTreePanelInput {
                    tree,
                    // Only a file tab's path belongs in the tree; the
                    // configuration tab (or any non-file tab) selects nothing.
                    active_file_path: if active_is_configuration {
                        None
                    } else {
                        buffer.path().map(|path| path.as_path())
                    },
                },
            ));
        }

        let central_fill = if active_is_configuration {
            ui.visuals().panel_fill
        } else {
            snapshot_color(highlight_snapshot.background, ui.visuals().code_bg_color)
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
                        workspace_path,
                        buffer,
                        tabs,
                        active_tab_id,
                        active_is_configuration,
                        configuration: self.configuration_view.as_mut(),
                        highlight_snapshot,
                        editor_config,
                        carets: &carets,
                        selection_ranges: &selection_ranges,
                        active_caret_index,
                        caret_visible,
                        sidebar_visible: self.sidebar_visible,
                        document_dirty,
                        document_status,
                        document_missing,
                        document_binary,
                        restoring_session,
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