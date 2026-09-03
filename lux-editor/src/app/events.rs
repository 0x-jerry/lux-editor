use super::{App, OpenDocument};
use crate::events::{
    AppEvent, ConfigurationEvent, CustomEvent, DocumentEvent, EditingEvent, ShellEvent,
    WorkspaceEvent,
};
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

    /// File system & workspace: raw mutations of the workspace tree.
    /// Workspace: mutations of the workspace tree and its refresh.
    fn handle_workspace_event(&mut self, event: WorkspaceEvent) {
        match event {
            WorkspaceEvent::FileChange => self.on_file_change(),
            WorkspaceEvent::Delete(path) => {
                if path.is_dir() {
                    std::fs::remove_dir_all(path).ok();
                } else {
                    std::fs::remove_file(path).ok();
                }
                self.on_file_change();
            }
            WorkspaceEvent::Rename(old, new) => {
                std::fs::rename(old, new).ok();
                self.on_file_change();
            }
            WorkspaceEvent::NewFile(parent) => {
                std::fs::File::create(parent.join("new_file.txt")).ok();
                self.on_file_change();
            }
            WorkspaceEvent::NewFolder(parent) => {
                std::fs::create_dir(parent.join("new_folder")).ok();
                self.on_file_change();
            }
        }
    }

    /// Document lifecycle & content pipeline: IO round-trips and tabs.
    fn handle_document_event(&mut self, event: DocumentEvent, ctx: &egui::Context) {
        match event {
            DocumentEvent::FileLoaded { path, buffer } => match buffer {
                Ok(buffer) => {
                    let next_doc = OpenDocument::from_buffer(buffer);
                    if self.should_reuse_active_document_slot() {
                        self.documents.tabs[self.documents.active_document] = next_doc;
                    } else {
                        self.documents.tabs.push(next_doc);
                        self.documents.active_document =
                            self.documents.tabs.len().saturating_sub(1);
                    }
                    self.documents.touch_caret_blink();
                    self.update_window_title(ctx);
                    self.track_file_open(&path);
                    self.refresh_language_intelligence();
                }
                Err(err) => {
                    self.active_document_mut().document_status =
                        Some(format!("Failed to open {}: {}", path.display(), err));
                }
            },
            DocumentEvent::FileSaved {
                path,
                generation,
                ok,
            } => {
                let active_document = self.active_document_mut();
                if ok {
                    if active_document.edit_generation == generation {
                        active_document.document_dirty = false;
                    }
                    active_document.document_status = Some(format!("Saved {}", path.display()));
                } else {
                    active_document.document_status = Some("Failed to save file".to_string());
                }
                self.update_window_title(ctx);
                self.track_file_open(&path);
                self.on_file_change();
            }
            DocumentEvent::FormattingFinished {
                generation,
                from_save,
                result,
            } => self.on_formatting_finished(generation, from_save, result, ctx),
            DocumentEvent::SwitchDocument(index) => self.switch_to_document(index, ctx),
            DocumentEvent::CloseDocument(index) => self.close_document(index, ctx),
            DocumentEvent::SaveFile => {
                self.save_current_buffer(ctx);
            }
            DocumentEvent::FormatFile => self.format_active_document(ctx),
        }
    }

    /// App-level state & navigation: config refresh and open commands.
    fn handle_app_event(&mut self, event: AppEvent, ctx: &egui::Context) {
        match event {
            AppEvent::ConfigChange => self.on_config_change(),
            AppEvent::OpenFile(path) => self.open_file(path, ctx),
            AppEvent::OpenFolder(path) => self.open_folder(path),
            AppEvent::ClearRecentItems => self.settings.editor_config.clear_recent_items(),
        }
    }

    /// Shell & navigation: view switching and title-bar actions.
    fn handle_shell_event(
        &mut self,
        event: ShellEvent,
        // macOS only consumes `ctx` in the platform-gated menu arm below.
        #[cfg_attr(target_os = "macos", allow(unused_variables))]
        ctx: &egui::Context,
    ) {
        match event {
            ShellEvent::SwitchToEditor => self.chrome.shell.switch_to_editor(),
            ShellEvent::SwitchToConfiguration => self.chrome.shell.switch_to_configuration(),
            ShellEvent::ToggleSidebar => self.chrome.shell.toggle_sidebar(),
            ShellEvent::ToggleCommandPanel => self.chrome.command_panel.toggle(),
            #[cfg(not(target_os = "macos"))]
            ShellEvent::TitleBarMenu(menu) => self.on_title_bar_menu(menu, ctx),
        }
    }

    /// Configuration: the configuration view autosave.
    fn handle_configuration_event(&mut self, event: ConfigurationEvent) {
        match event {
            ConfigurationEvent::ConfigurationSaved(settings) => {
                self.apply_saved_configuration(settings)
            }
        }
    }

    /// Text editing & caret: pointer interaction.
    fn handle_editing_event(&mut self, event: EditingEvent) {
        match event {
            EditingEvent::SetCaretFromPointer {
                line_index,
                column,
                selecting,
                add_cursor,
            } => self.set_caret_from_pointer(line_index, column, selecting, add_cursor),
            EditingEvent::SelectWordFromPointer { line_index, column } => {
                self.select_word_from_pointer(line_index, column)
            }
        }
    }
}
