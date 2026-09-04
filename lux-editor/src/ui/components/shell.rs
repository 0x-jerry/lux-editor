//! The app shell component: chrome (title/status bars), sidebar and the
//! active view. Owns the shell navigation state and its child components.

use super::{
    ConfigurationView, ConfigurationViewInput, EditorView, EditorViewState, FileTreePanel,
    FileTreePanelInput,
};
use crate::app::ShellView;
use crate::config::{Config, EditorSettings};
use crate::events::CustomEvent;
use crate::file_tree::FileTree;
use crate::language::HighlightSnapshot;
use crate::ui::component::Component;
use crate::ui::highlight::snapshot_color;
use crate::ui::widgets::{
    StatusBar, StatusBarData, StatusBarSection, TitleBar, TitleBarData, window_resize_handle,
};
use eframe::egui;
use lux_core::Buffer;
use std::ops::Range;
use std::path::PathBuf;

/// The document-model snapshot the shell renders from each frame.
pub struct ShellInput<'a> {
    pub file_tree: Option<&'a FileTree>,
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a Buffer,
    pub document_tabs: &'a [crate::ui::types::DocumentTab],
    pub active_document_index: usize,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    pub document_status: Option<&'a str>,
    /// All cursor positions as 1-based (line, column).
    pub carets: Vec<(usize, usize)>,
    pub selection_ranges: Vec<Range<usize>>,
    pub active_caret_index: usize,
    pub caret_visible: bool,
    pub document_dirty: bool,
}

/// The app shell: chrome (title/status bars), sidebar and the active view.
/// Owns the shell navigation state and its child components.
pub struct Shell {
    shell_view: ShellView,
    sidebar_visible: bool,
    title_bar: TitleBar,
    status_bar: StatusBar,
    file_tree_panel: FileTreePanel,
    editor_view: EditorView,
    configuration_view: ConfigurationView,
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            shell_view: ShellView::Editor,
            sidebar_visible: true,
            title_bar: TitleBar,
            status_bar: StatusBar,
            file_tree_panel: FileTreePanel::default(),
            editor_view: EditorView,
            configuration_view: ConfigurationView::default(),
        }
    }
}

impl Shell {
    pub fn shell_view(&self) -> ShellView {
        self.shell_view
    }

    pub fn switch_to_editor(&mut self) {
        self.shell_view = ShellView::Editor;
    }

    pub fn switch_to_configuration(&mut self) {
        self.shell_view = ShellView::Configuration;
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    pub fn sync_config_draft(&mut self, settings: &EditorSettings) {
        self.configuration_view.sync_draft(settings);
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
            document_tabs,
            active_document_index,
            highlight_snapshot,
            editor_config,
            document_status,
            carets,
            selection_ranges,
            active_caret_index,
            caret_visible,
            document_dirty,
        } = state;

        let mut events = Vec::new();

        self.title_bar.render(
            ui,
            TitleBarData {
                app_title: "Lux",
            },
        );

        let selection_len: usize = selection_ranges
            .iter()
            .map(|range| range.end - range.start)
            .sum();
        let (caret_line, caret_column) = carets.get(active_caret_index).copied().unwrap_or((1, 1));
        let section = match self.shell_view {
            ShellView::Editor => StatusBarSection::Editor {
                caret_line,
                caret_column,
                selection_len,
                document_dirty,
                document_status,
            },
            ShellView::Configuration => StatusBarSection::Configuration {
                config_status: self.configuration_view.status(),
            },
        };
        let right_label = match self.shell_view {
            ShellView::Editor => buffer
                .path()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "Untitled".to_string()),
            ShellView::Configuration => Config::user_settings_path().display().to_string(),
        };
        self.status_bar.render(
            ui,
            StatusBarData {
                mode_label: match self.shell_view {
                    ShellView::Editor => "EDITOR",
                    ShellView::Configuration => "CONFIGURATION",
                },
                section,
                right_label: &right_label,
            },
        );

        if self.shell_view == ShellView::Editor
            && self.sidebar_visible
            && let Some(tree) = file_tree
        {
            events.extend(self.file_tree_panel.render(
                ui,
                FileTreePanelInput {
                    tree,
                    workspace_path: workspace_path.map(|path| path.as_path()),
                    active_file_path: buffer.path().map(|path| path.as_path()),
                },
            ));
        }

        let central_fill = if self.shell_view == ShellView::Editor {
            snapshot_color(highlight_snapshot.background, ui.visuals().code_bg_color)
        } else {
            ui.visuals().panel_fill
        };
        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(ui.style())
                    .fill(central_fill)
                    .inner_margin(0),
            )
            .show(ui, |ui| {
                if self.shell_view == ShellView::Editor {
                    events.extend(self.editor_view.render(
                        ui,
                        EditorViewState {
                            workspace_path,
                            buffer,
                            document_tabs,
                            active_document_index,
                            highlight_snapshot,
                            editor_config,
                            carets: &carets,
                            selection_ranges: &selection_ranges,
                            active_caret_index,
                            caret_visible,
                        },
                    ));
                } else {
                    events.extend(
                        self.configuration_view
                            .render(
                                ui,
                                ConfigurationViewInput {
                                    workspace_path,
                                    buffer,
                                    editor_config,
                                },
                            )
                            .into_iter()
                            .map(CustomEvent::Configuration),
                    );
                }
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
