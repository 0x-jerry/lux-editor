use super::{App, ShellView};
use crate::ui;
use eframe::{App as EframeApp, Frame, egui};

impl EframeApp for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
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
            },
        );
        for event in events {
            self.handle_event(event, ctx);
        }

        ctx.request_repaint();
    }
}
