use super::App;
use crate::events::CustomEvent;
use eframe::egui;

impl App {
    pub(super) fn process_pending_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.runtime.event_rx.try_recv() {
            self.handle_event(event, ctx);
        }
    }

    pub(super) fn handle_event(&mut self, event: CustomEvent, ctx: &egui::Context) {
        match event {
            CustomEvent::Workspace(event) => self.handle_workspace_event(event),
            CustomEvent::Document(event) => self.handle_document_event(event, ctx),
            CustomEvent::App(event) => self.handle_app_event(event, ctx),
            CustomEvent::Shell(event) => self.handle_shell_event(event, ctx),
            CustomEvent::Configuration(event) => self.handle_configuration_event(event),
            CustomEvent::Editing(event) => self.handle_editing_event(event),
        }
    }

}
