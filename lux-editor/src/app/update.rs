use super::App;
use crate::ui;
use crate::ui::Component;
use crate::ui::theme::{self, ThemeChoice};
use eframe::{App as EframeApp, Frame, egui};

impl EframeApp for App {
    /// Pre-UI pass: process events/input and mutate editor state before the
    /// frame is rendered. Painting is not allowed here.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        #[cfg(target_os = "macos")]
        if ctx.input(|input| input.viewport().close_requested()) {
            std::process::exit(0);
        }

        self.process_pending_events(ctx);

        // Native menubar/tray events flow through the same command pipeline as
        // the rendered chrome.
        self.chrome.native.install();
        self.chrome.native.update_tray_label();
        let native_commands = self.chrome.native.drain();
        for command in native_commands {
            self.on_title_bar_menu(command, ctx);
        }

        self.highlighting.service.update();
        self.handle_keyboard_input(ctx);
        self.flush_scheduled_language_refresh();

        let toggle_sidebar = ctx.input_mut(|input| {
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
            ctx.system_theme(),
        );
        self.chrome.needs_style_refresh |= self.chrome.runtime_theme != Some(resolved);

        if self.chrome.needs_style_refresh {
            self.chrome.needs_style_refresh = false;
            self.apply_style(ctx, resolved);
            // Probed after apply_style: it derives from the runtime_theme that
            // call just stored. Font-only changes must not force a re-parse.
            if self.syntax_theme_changed() {
                self.refresh_language_intelligence();
            }
        }
    }

    /// Render pass: snapshot the document state and hand the whole frame to
    /// the [`crate::ui::components::app_view::AppView`] component. The events
    /// it emits are dispatched to the app's reducer.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut Frame) {
        let ctx = ui.ctx().clone();

        let highlight_snapshot = self.highlight_snapshot().clone();
        let (carets, active_caret_index, selection_ranges) = {
            let active_document = &self.documents.tabs[self.documents.active_document];
            let caret_state = &active_document.caret_state;
            let carets = (0..caret_state.len())
                .map(|index| {
                    lux_core::editor::line_column(
                        &active_document.buffer,
                        caret_state.caret_char_at(index),
                    )
                })
                .collect::<Vec<_>>();
            let active_caret_index = caret_state.active_index();
            let selection_ranges = caret_state.selection_ranges();
            (carets, active_caret_index, selection_ranges)
        };
        let caret_visible = self.documents.caret_blink_visible();
        let document_tabs = self
            .documents
            .tabs
            .iter()
            .map(|document| ui::DocumentTab {
                title: document.title(),
                dirty: document.document_dirty,
            })
            .collect::<Vec<_>>();
        let active_document = &self.documents.tabs[self.documents.active_document];
        let events = {
            let mut view = ui::AppView;
            view.render(
                ui,
                ui::AppViewInput {
                    shell: &mut self.chrome.shell,
                    command_panel: &mut self.chrome.command_panel,
                    about_window: &mut self.chrome.about_window,
                    file_tree: self.workspace.file_tree.as_ref(),
                    workspace_path: self.workspace.path.as_ref(),
                    buffer: &active_document.buffer,
                    document_tabs: &document_tabs,
                    active_document_index: self.documents.active_document,
                    highlight_snapshot: &highlight_snapshot,
                    editor_config: &self.settings.editor_config,
                    document_status: active_document.document_status.as_deref(),
                    carets,
                    selection_ranges,
                    active_caret_index,
                    caret_visible,
                    document_dirty: active_document.document_dirty,
                },
            )
        };
        for event in events {
            self.handle_event(event, &ctx);
        }

        ctx.request_repaint();
    }
}
