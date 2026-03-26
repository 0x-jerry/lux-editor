use super::{App, ShellView};
use crate::events::CustomEvent;
use eframe::egui;

impl App {
    pub(super) fn process_pending_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.event_rx.try_recv() {
            self.handle_event(event, ctx);
        }
    }

    pub(super) fn handle_event(&mut self, event: CustomEvent, ctx: &egui::Context) {
        match event {
            CustomEvent::FileChange => self.on_file_change(),
            CustomEvent::ConfigChange => self.on_config_change(),
            CustomEvent::OpenFile(path) => self.open_file(path, ctx),
            CustomEvent::OpenFolder(path) => self.open_folder(path),
            CustomEvent::Delete(path) => {
                if path.is_dir() {
                    std::fs::remove_dir_all(path).ok();
                } else {
                    std::fs::remove_file(path).ok();
                }
                self.on_file_change();
            }
            CustomEvent::Rename(old, new) => {
                std::fs::rename(old, new).ok();
                self.on_file_change();
            }
            CustomEvent::NewFile(parent) => {
                std::fs::File::create(parent.join("new_file.txt")).ok();
                self.on_file_change();
            }
            CustomEvent::NewFolder(parent) => {
                std::fs::create_dir(parent.join("new_folder")).ok();
                self.on_file_change();
            }
            CustomEvent::SwitchToEditor => self.shell_view = ShellView::Editor,
            CustomEvent::SwitchToConfiguration => self.shell_view = ShellView::Configuration,
            CustomEvent::ConfigurationDraftChanged => self.schedule_configuration_autosave(),
            CustomEvent::SetCaretFromPointer {
                line_index,
                column,
                selecting,
            } => self.set_caret_from_pointer(line_index, column, selecting),
        }
    }

    pub(super) fn on_file_change(&mut self) {
        if let Some(path) = &self.workspace_path {
            self.file_tree = Some(crate::file_tree::FileTree::new(path));
        }
    }

    pub(super) fn on_config_change(&mut self) {
        if self.editor_config.reload_settings() {
            self.needs_style_refresh = true;
            self.refresh_language_intelligence();
        }
        self.config_draft = self.editor_config.settings.clone();
        self.config_status = None;
        self.config_autosave_deadline = None;
    }
}
