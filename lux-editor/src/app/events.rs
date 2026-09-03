use super::{App, OpenDocument, ShellView};
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
            CustomEvent::FileLoaded { path, buffer } => match buffer {
                Ok(buffer) => {
                    let next_doc = OpenDocument::from_buffer(buffer);
                    if self.should_reuse_active_document_slot() {
                        self.documents[self.active_document] = next_doc;
                    } else {
                        self.documents.push(next_doc);
                        self.active_document = self.documents.len().saturating_sub(1);
                    }
                    self.touch_caret_blink();
                    self.update_window_title(ctx);
                    self.track_file_open(&path);
                    self.refresh_language_intelligence();
                }
                Err(err) => {
                    self.active_document_mut().document_status =
                        Some(format!("Failed to open {}: {}", path.display(), err));
                }
            },
            CustomEvent::FileSaved {
                path,
                generation,
                ok,
            } => {
                let active_document = self.active_document_mut();
                if ok {
                    if active_document.edit_generation == generation {
                        active_document.document_dirty = false;
                    }
                    active_document.document_status =
                        Some(format!("Saved {}", path.display()));
                } else {
                    active_document.document_status =
                        Some("Failed to save file".to_string());
                }
                self.update_window_title(ctx);
                self.track_file_open(&path);
                self.on_file_change();
            }
            CustomEvent::FormattingFinished {
                generation,
                from_save,
                result,
            } => self.on_formatting_finished(generation, from_save, result, ctx),
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
            CustomEvent::ClearRecentItems => self.editor_config.clear_recent_items(),
            CustomEvent::SwitchDocument(index) => self.switch_to_document(index, ctx),
            CustomEvent::CloseDocument(index) => self.close_document(index, ctx),
            CustomEvent::SwitchToEditor => self.shell_view = ShellView::Editor,
            CustomEvent::SwitchToConfiguration => self.shell_view = ShellView::Configuration,
            CustomEvent::ToggleSidebar => self.sidebar_visible = !self.sidebar_visible,
            CustomEvent::TitleBarMenu(menu) => self.on_title_bar_menu(menu, ctx),
            CustomEvent::ConfigurationDraftChanged => self.schedule_configuration_autosave(),
            CustomEvent::SetCaretFromPointer {
                line_index,
                column,
                selecting,
                add_cursor,
            } => self.set_caret_from_pointer(line_index, column, selecting, add_cursor),
            CustomEvent::SelectWordFromPointer { line_index, column } => {
                self.select_word_from_pointer(line_index, column)
            }
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

    fn on_title_bar_menu(
        &mut self,
        menu: crate::ui::kit::title_bar::TitleBarMenu,
        ctx: &egui::Context,
    ) {
        use crate::app::input::EditorCommand;
        use crate::ui::kit::title_bar::TitleBarMenu;
        match menu {
            TitleBarMenu::OpenFile => {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.open_file(path, ctx);
                }
            }
            TitleBarMenu::OpenFolder => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.open_folder(path);
                }
            }
            TitleBarMenu::SaveFile => {
                self.save_current_buffer(ctx);
            }
            TitleBarMenu::Undo => {
                self.execute_command(EditorCommand::Undo, ctx);
            }
            TitleBarMenu::Redo => {
                self.execute_command(EditorCommand::Redo, ctx);
            }
            TitleBarMenu::Cut => {
                self.execute_command(EditorCommand::Cut, ctx);
            }
            TitleBarMenu::Copy => {
                self.execute_command(EditorCommand::Copy, ctx);
            }
            TitleBarMenu::Paste => {
                if let Some(text) = clipboard_text() {
                    self.execute_command(EditorCommand::Paste(text), ctx);
                }
            }
            TitleBarMenu::SelectAll => {
                self.execute_command(EditorCommand::SelectAll, ctx);
            }
            TitleBarMenu::CommandPalette => self.toggle_command_panel(),
            TitleBarMenu::SwitchToEditor => self.shell_view = ShellView::Editor,
            TitleBarMenu::SwitchToConfiguration => self.shell_view = ShellView::Configuration,
            TitleBarMenu::ToggleSidebar => self.sidebar_visible = !self.sidebar_visible,
            TitleBarMenu::About => self.about_open = true,
        }
    }
}

fn clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}
