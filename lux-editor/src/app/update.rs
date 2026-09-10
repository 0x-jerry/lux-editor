//! The eframe frame adapter: `logic` (pre-UI state pass) and `ui` (render
//! pass). The logic pass borrows every domain through one [`Ctx`] and runs the
//! per-frame steps; the render pass hands the frame to the `AppView` component
//! and dispatches the messages it emits.

use super::Ctx;
use crate::app::App;
use crate::chrome;
use crate::component::Component;
use crate::settings::configuration_view::ConfigurationView;
use crate::theme::{self, ThemeChoice};
use eframe::{App as EframeApp, Frame, egui};
use std::time::Duration;

impl EframeApp for App {
    /// Pre-UI pass: process events/input and mutate editor state before the
    /// frame is rendered. Painting is not allowed here.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        #[cfg(target_os = "macos")]
        if ctx.input(|input| input.viewport().close_requested()) {
            self.settings.editor_config.flush_recent();
            std::process::exit(0);
        }
        self.ctx().frame_logic();
    }

    /// Render pass: snapshot the document state and hand the whole frame to
    /// the [`crate::chrome::ui::app_view::AppView`] component. The events
    /// it emits are dispatched to the app's actions.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
        let ctx = ui.ctx().clone();

        let editor_focused = self.ctx().editor_focused();
        let highlight_snapshot = self.highlighting.service.snapshot();
        let active_document = self
            .tabs
            .active_text()
            .or_else(|| self.tabs.first_text())
            .expect("a text tab always exists");
        let (carets, active_caret_index, selection_ranges) = {
            let caret_state = &active_document.caret_state;
            let carets = (0..caret_state.len())
                .map(|index| {
                    crate::document::line_column(
                        &active_document.buffer,
                        caret_state.caret_char_at(index),
                    )
                })
                .collect::<Vec<_>>();
            let active_caret_index = caret_state.active_index();
            let selection_ranges = caret_state.selection_ranges();
            (carets, active_caret_index, selection_ranges)
        };
        let caret_visible = if editor_focused {
            self.tabs.caret_blink_visible()
        } else {
            false // hide the caret entirely while the editor isn't focused
        };
        let tabs = self
            .tabs
            .tabs
            .iter()
            .map(|tab| tab.meta())
            .collect::<Vec<_>>();
        let active_tab_id = self.tabs.tabs[self.tabs.active_tab].id;
        let active_is_markdown = !self.tabs.active_is_configuration()
            && crate::highlighting::LanguageKind::from_path(
                active_document.buffer.path().map(|path| &**path),
            ) == crate::highlighting::LanguageKind::Markdown;
        let events = {
            let mut view = chrome::AppView;
            view.render(
                ui,
                chrome::AppViewInput {
                    shell: &mut self.chrome.shell,
                    command_panel: &mut self.chrome.command_panel,
                    about_window: &mut self.chrome.about_window,
                    file_tree: self.workspace.file_tree.as_mut(),
                    workspace_path: self.workspace.path.as_ref(),
                    buffer: &active_document.buffer,
                    tabs: &tabs,
                    active_tab_id,
                    active_is_configuration: self.tabs.active_is_configuration(),
                    highlight_snapshot,
                    editor_config: &self.settings.editor_config,
                    document_status: active_document.document_status.as_deref(),
                    restoring_session: self.tabs.pending_loads > 0,
                    carets,
                    selection_ranges,
                    active_caret_index,
                    caret_visible,
                    document_dirty: active_document.document_dirty,
                    document_missing: active_document.missing,
                    document_binary: active_document.binary,
                    document_file_size: active_document.last_disk_stat.map(|(len, _)| len),
                    active_is_markdown,
                },
            )
        };
        {
            let mut cx = self.ctx();
            for event in events {
                cx.handle_event(event);
            }
        }

        crate::app::startup::stage_once!("first frame presented");

        // Everything repaints on input; background events wake the loop via
        // `Runtime::ctx`; only the caret blink needs a steady tick here, and
        // only while the editor is focused.
        if editor_focused {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }
}

impl Ctx<'_> {
    /// The whole pre-UI logic pass, in the order it has to run. Order is
    /// load-bearing in places; see the comments on the individual steps.
    fn frame_logic(&mut self) {
        // Workspace/document setup waits for a painted window: everything
        // below touches the disk and must not delay first-frame presentation.
        // `runtime_theme` is set by the first `apply_style`, so this fires on
        // the second logic pass — after the first frame has been presented.
        // Runs before event processing so a first-frame open (synthetic or
        // otherwise) is not clobbered by the CLI path.
        if !self.frame.deferred_init_done {
            if self.chrome.runtime_theme.is_some() {
                self.frame.deferred_init_done = true;
                self.restart_settings_watcher();
                if self.workspace.path.is_none() {
                    let initial_path = self.frame.pending_init.take();
                    self.initialize_from_path(initial_path);
                }
            } else {
                // Occluded/minimized windows never run `ui`, so drive the
                // second logic pass from here.
                self.egui_ctx().request_repaint();
            }
        }

        self.process_pending_events();

        // Keep the shell's configuration session in step with the tab: created
        // synced to current settings when the tab opens, dropped when it closes.
        let has_configuration_tab = self.tabs.has_configuration_tab();
        if has_configuration_tab && self.chrome.shell.configuration_view.is_none() {
            let mut view = ConfigurationView::default();
            view.sync_draft(&self.settings.editor_config.settings);
            self.chrome.shell.configuration_view = Some(view);
        } else if !has_configuration_tab && self.chrome.shell.configuration_view.is_some() {
            self.chrome.shell.configuration_view = None;
        }
        self.sync_workspace_session();
        self.flush_recent_config();

        // Native menubar/tray events flow through the same command pipeline as
        // the rendered chrome.
        self.native_menu_pass();

        self.highlighting.service.update();
        self.handle_keyboard_input();
        self.flush_scheduled_language_refresh();

        let toggle_sidebar = self.egui_ctx().input_mut(|input| {
            input.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::B,
            ))
        });
        if toggle_sidebar {
            self.chrome.shell.toggle_sidebar();
        }

        // Live `Auto` following: whatever the config plus the OS report resolve to
        // is what has to be on screen. Probed after the handlers so a config
        // change they applied lands in this same pass.
        let resolved = theme::resolve(
            ThemeChoice::from_value(&self.settings.editor_config.settings.theme.choice),
            self.egui_ctx().system_theme(),
        );
        self.chrome.needs_style_refresh |= self.chrome.runtime_theme != Some(resolved);

        if self.chrome.needs_style_refresh {
            self.chrome.needs_style_refresh = false;
            self.apply_style(resolved);
            // Probed after apply_style: it derives from the runtime_theme that
            // call just stored. Font-only changes must not force a re-parse.
            if self.syntax_colors_changed() {
                self.refresh_language_intelligence();
            }
        }
        crate::app::startup::stage_once!("first logic pass");
    }
}
