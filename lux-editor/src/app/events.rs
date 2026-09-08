//! Event dispatch. The event-bus envelope routes each domain's event to the
//! matching `Ctx` action (see `app/actions`). Background workers report
//! through the channel; this is where the frame drains them.

use super::Ctx;
use crate::events::CustomEvent;

impl Ctx<'_> {
    pub(super) fn process_pending_events(&mut self) {
        while let Ok(event) = self.runtime.event_rx.try_recv() {
            self.handle_event(event);
        }
    }

    pub(super) fn handle_event(&mut self, event: CustomEvent) {
        match event {
            CustomEvent::Workspace(event) => self.handle_workspace_event(event),
            CustomEvent::Document(event) => self.handle_document_event(event),
            CustomEvent::App(event) => self.handle_app_event(event),
            CustomEvent::Shell(event) => self.handle_shell_event(event),
            CustomEvent::Configuration(event) => self.handle_configuration_event(event),
            CustomEvent::Editing(event) => self.handle_editing_event(event),
        }
    }
}
