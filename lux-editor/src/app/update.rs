use super::App;
use crate::ui;
use eframe::{App as EframeApp, Frame, egui};

impl EframeApp for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.process_pending_events(ctx);
        self.highlighting_service.update();
        self.handle_keyboard_input(ctx);

        if self.needs_style_refresh {
            self.apply_editor_settings(ctx);
            self.needs_style_refresh = false;
        }

        let events = ui::draw_ui(
            ctx,
            self.file_tree.as_ref(),
            self.workspace_path.as_ref(),
            &self.buffer,
            self.highlight_snapshot(),
            &self.editor_config,
        );
        for event in events {
            self.handle_event(event, ctx);
        }

        ctx.request_repaint();
    }
}
