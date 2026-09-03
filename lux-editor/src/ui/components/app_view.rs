//! Root view component: renders the shell plus the overlay windows (command
//! palette, about dialog) and reports every effect they requested as events.

use crate::config::Config;
use crate::events::CustomEvent;
use crate::file_tree::FileTree;
use crate::language::HighlightSnapshot;
use crate::ui::component::Component;
use crate::ui::components::about::AboutWindow;
use crate::ui::components::command_panel::CommandPanel;
use crate::ui::components::shell::{Shell, ShellInput};
use crate::ui::types::DocumentTab;
use eframe::egui;
use lux_core::Buffer;
use std::ops::Range;
use std::path::PathBuf;

/// Everything the root view needs this frame: the components it renders
/// (mutable, borrowed from the app) plus the document/workspace snapshot.
pub struct AppViewInput<'a> {
    pub shell: &'a mut Shell,
    pub command_panel: &'a mut CommandPanel,
    pub about_window: &'a mut AboutWindow,
    pub file_tree: Option<&'a FileTree>,
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a Buffer,
    pub document_tabs: &'a [DocumentTab],
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

/// The app's view root. Owns no state itself; the shell and overlay
/// components keep theirs and are borrowed in per frame.
pub struct AppView;

impl Component for AppView {
    type Message = CustomEvent;
    type Input<'a> = AppViewInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<CustomEvent> {
        let AppViewInput {
            shell,
            command_panel,
            about_window,
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
        } = input;

        let mut events = shell.render(
            ui,
            ShellInput {
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
            },
        );

        // Overlays render after the main layout so they stay on top.
        events.extend(command_panel.render(ui, editor_config));
        about_window.render(ui, ());

        events
    }
}
