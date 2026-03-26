use super::{App, ShellView};
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
        if self.shell_view == ShellView::Editor {
            self.handle_keyboard_input(ctx);
        }

        if self.needs_style_refresh {
            self.apply_editor_settings(ctx);
            self.needs_style_refresh = false;
        }

        let highlight_snapshot = self.highlight_snapshot().clone();
        let (caret_line, caret_column) = self.caret_position();
        let selection_len = self.selection_len();
        let caret_visible = self.caret_blink_visible();
        let events = ui::draw_ui(
            ctx,
            ui::DrawUiState {
                file_tree: self.file_tree.as_ref(),
                workspace_path: self.workspace_path.as_ref(),
                buffer: &self.buffer,
                highlight_snapshot: &highlight_snapshot,
                editor_config: &self.editor_config,
                config_draft: &mut self.config_draft,
                config_status: self.config_status.as_deref(),
                shell_view: self.shell_view,
                caret_line,
                caret_column,
                selection_len,
                caret_visible,
            },
        );
        for event in events {
            self.handle_event(event, ctx);
        }
        self.flush_configuration_autosave();

        ctx.request_repaint();
    }
}
