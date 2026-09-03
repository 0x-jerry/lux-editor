use super::App;
use crate::ui;
use eframe::{App as EframeApp, Frame, egui};

impl EframeApp for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.last_system_theme = ctx.system_theme();

        #[cfg(target_os = "macos")]
        if ctx.input(|input| input.viewport().close_requested()) {
            std::process::exit(0);
        }

        self.process_pending_events(ctx);
        self.highlighting_service.update();
        self.handle_keyboard_input(ctx);
        self.flush_scheduled_language_refresh();

        if self.applied_theme_dark.is_some()
            && crate::ui::theme::ThemeChoice::from_value(
                &self.editor_config.settings.theme.choice,
            ) == crate::ui::theme::ThemeChoice::Auto
            && self.last_system_theme.map(|theme| theme == egui::Theme::Dark)
                != self.applied_theme_dark
        {
            self.needs_style_refresh = true;
            self.refresh_language_intelligence();
        }

        let toggle_sidebar = ctx.input_mut(|input| {
            input.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::B,
            ))
        });
        if toggle_sidebar {
            self.sidebar_visible = !self.sidebar_visible;
        }

        if self.needs_style_refresh {
            self.apply_editor_settings(ctx);
            self.needs_style_refresh = false;
        }

        let highlight_snapshot = self.highlight_snapshot().clone();
        let (carets, active_caret_index, selection_ranges) = {
            let active_document = &self.documents[self.active_document];
            let caret_state = &active_document.caret_state;
            let carets = (0..caret_state.len())
                .map(|index| {
                    crate::app::editor::line_column(&active_document.buffer, caret_state.caret_char_at(index))
                })
                .collect::<Vec<_>>();
            let active_caret_index = caret_state.active_index();
            let selection_ranges = caret_state.selection_ranges();
            (carets, active_caret_index, selection_ranges)
        };
        let caret_visible = self.caret_blink_visible();
        let reveal_active_in_tree = self.reveal_active_in_tree;
        let document_tabs = self
            .documents
            .iter()
            .map(|document| ui::DocumentTab {
                title: document.title(),
                dirty: document.document_dirty,
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
                sidebar_visible: self.sidebar_visible,
                carets,
                selection_ranges,
                active_caret_index,
                caret_visible,
                document_dirty: active_document.document_dirty,
            },
        );
        self.reveal_active_in_tree = false;
        for event in events {
            self.handle_event(event, ctx);
        }
        self.render_command_panel(ctx);
        self.render_about_window(ctx);
        self.flush_configuration_autosave();

        ctx.request_repaint();
    }
}

impl App {
    fn render_about_window(&mut self, ctx: &egui::Context) {
        if !self.about_open {
            return;
        }
        egui::Window::new("About Lux")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.heading("Lux Editor");
                ui.label("A fast Rust editor focused on large-file performance.");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(10.0);
                if ui.button("Close").clicked() {
                    self.about_open = false;
                }
            });
    }
}
