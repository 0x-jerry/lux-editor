use super::App;
use crate::ui;
use eframe::{App as EframeApp, Frame, egui};

impl EframeApp for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        #[cfg(target_os = "macos")]
        if ctx.input(|input| input.viewport().close_requested()) {
            std::process::exit(0);
        }

        self.process_pending_events(ctx);
        self.highlighting_service.update();
        self.handle_keyboard_input(ctx);

        if self.needs_style_refresh {
            self.apply_editor_settings(ctx);
            self.needs_style_refresh = false;
        }

        let highlight_snapshot = self.highlight_snapshot().clone();
        let (caret_line, caret_column) = self.caret_position();
        let selection_len = self.selection_len();
        let caret_visible = self.caret_blink_visible();
        let reveal_active_in_tree = self.reveal_active_in_tree;
        let document_tabs = self
            .documents
            .iter()
            .map(|document| ui::DocumentTab {
                title: document.title(),
            })
            .collect::<Vec<_>>();
        let active_document = &self.documents[self.active_document];
        let events = ui::draw_ui(
            ctx,
            ui::DrawUiState {
                file_tree: self.file_tree.as_ref(),
                workspace_path: self.workspace_path.as_ref(),
                buffer: &active_document.buffer,
                document_tabs: &document_tabs,
                active_document_index: self.active_document,
                highlight_snapshot: &highlight_snapshot,
                editor_config: &self.editor_config,
                config_draft: &mut self.config_draft,
                config_status: self.config_status.as_deref(),
                document_status: active_document.document_status.as_deref(),
                shell_view: self.shell_view,
                reveal_active_in_tree,
                caret_line,
                caret_column,
                selection_len,
                caret_visible,
                document_dirty: active_document.document_dirty,
            },
        );
        self.reveal_active_in_tree = false;
        for event in events {
            self.handle_event(event, ctx);
        }
        self.render_command_panel(ctx);
        self.flush_configuration_autosave();

        ctx.request_repaint();
    }
}
