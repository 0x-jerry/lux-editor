//! Root view component: renders the shell plus the overlay windows (command
//! palette, about dialog) and reports every effect they requested as events.

use super::about::AboutWindow;
use super::command_panel::{CommandPanel, CommandPanelInput, PaletteContext};
use super::shell::{Shell, ShellInput};
use crate::component::Component;
use crate::document::DocumentBuffer;
use crate::events::CustomEvent;
use crate::highlighting::HighlightSnapshot;
use crate::settings::Config;
use crate::workspace::FileTree;
use eframe::egui;
use std::ops::Range;
use std::path::PathBuf;

/// Everything the root view needs this frame: the components it renders
/// (mutable, borrowed from the app) plus the document/workspace snapshot.
pub struct AppViewInput<'a> {
    pub shell: &'a mut Shell,
    pub command_panel: &'a mut CommandPanel,
    pub about_window: &'a mut AboutWindow,
    pub file_tree: Option<&'a mut FileTree>,
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a DocumentBuffer,
    pub tabs: &'a [crate::tabs::TabMeta],
    pub active_tab_id: u64,
    pub active_is_configuration: bool,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    pub document_status: Option<&'a str>,
    /// Still reading the files a restored workspace remembered; the welcome page waits.
    pub restoring_session: bool,
    /// All cursor positions as 1-based (line, column).
    pub carets: Vec<(usize, usize)>,
    pub selection_ranges: Vec<Range<usize>>,
    pub active_caret_index: usize,
    pub caret_visible: bool,
    pub document_dirty: bool,
    pub document_missing: bool,
    pub document_binary: bool,
    /// The active tab is a markdown document (and not the configuration tab).
    pub active_is_markdown: bool,
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
            active_is_markdown,
        } = input;

        let mut events = shell.render(
            ui,
            ShellInput {
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
                active_is_markdown,
            },
        );

        // Overlays render after the main layout so they stay on top.
        events.extend(command_panel.render(
            ui,
            CommandPanelInput {
                config: editor_config,
                context: PaletteContext { active_is_markdown },
            },
        ));
        about_window.render(ui, ());

        events
    }
}
