use super::{App, OpenDocument, ShellView};
use crate::file_tree::FileTree;
use crate::language::{HighlightSnapshot, HighlightThemeConfig, LanguageKind};
use eframe::egui;
use lux_core::Buffer;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

impl App {
    pub fn new() -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (event_tx, event_rx) = mpsc::channel();
        let editor_config = crate::config::Config::load();
        let mut app = Self {
            rt,
            event_tx,
            event_rx,
            documents: vec![OpenDocument::new_empty()],
            active_document: 0,
            workspace_path: None,
            file_tree: None,
            workspace_watcher: None,
            settings_watcher: None,
            config_draft: editor_config.settings.clone(),
            editor_config,
            config_status: None,
            config_autosave_deadline: None,
            highlighting_service: crate::language::HighlightingService::new(),
            needs_style_refresh: true,
            reveal_active_in_tree: false,
            shell_view: ShellView::Editor,
            command_panel: Default::default(),
            caret_blink_anchor: std::time::Instant::now(),
        };
        let initial_path = std::env::args().nth(1).map(PathBuf::from);
        app.initialize_from_path(initial_path);
        app.editor_config.reload_settings();
        app.config_draft = app.editor_config.settings.clone();
        app.restart_settings_watcher();
        app.refresh_language_intelligence();
        app
    }

    pub(super) fn open_folder(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        self.workspace_path = Some(path.clone());
        self.file_tree = Some(FileTree::new(&path));
        self.editor_config.add_recent(path.clone(), true);
        self.workspace_watcher = Self::start_workspace_watcher(&path, self.event_tx.clone());
        self.reset_editor_state();
        self.open_workspace_last_file(&path);
        self.reveal_active_in_tree = true;
        self.restart_settings_watcher();
        if self.editor_config.reload_settings() {
            self.needs_style_refresh = true;
            self.refresh_language_intelligence();
        }
        self.config_draft = self.editor_config.settings.clone();
        self.config_status = None;
    }

    pub(super) fn open_file(&mut self, path: PathBuf, ctx: &egui::Context) {
        let path = path.canonicalize().unwrap_or(path);
        if let Some(index) = self.documents.iter().position(|doc| {
            doc.buffer
                .path()
                .is_some_and(|existing_path| existing_path == &path)
        }) {
            self.switch_to_document(index, ctx);
            return;
        }

        if let Ok(buffer) = self.rt.block_on(Buffer::from_file(&path)) {
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
    }

    pub(super) fn save_current_buffer(&mut self, ctx: &egui::Context) -> bool {
        let active_path = self.active_document().buffer.path().cloned();
        if active_path.is_none() {
            if let Some(path) = rfd::FileDialog::new().save_file() {
                self.buffer_mut().set_path(&path);
            } else {
                self.active_document_mut().document_status = Some("Save cancelled".to_string());
                return false;
            }
        }

        let save_result = {
            let active_index = self.active_document;
            let documents = &mut self.documents;
            let buffer = &mut documents[active_index].buffer;
            self.rt.block_on(buffer.save())
        };

        if save_result.is_ok() {
            let saved_path = self.buffer().path().cloned().unwrap();
            let active_document = self.active_document_mut();
            active_document.document_dirty = false;
            active_document.document_status = Some(format!("Saved {}", saved_path.display()));
            self.track_file_open(&saved_path);
            self.update_window_title(ctx);
            self.on_file_change();
            true
        } else {
            self.active_document_mut().document_status = Some("Failed to save file".to_string());
            false
        }
    }

    pub(super) fn refresh_language_intelligence(&mut self) {
        self.highlighting_service.set_theme(HighlightThemeConfig {
            theme_name: self.editor_config.settings.theme.syntax_theme.clone(),
            theme_path: self.editor_config.settings.theme.theme_path.clone(),
        });
        let language = LanguageKind::from_path(self.buffer().path().map(|v| &**v));
        self.highlighting_service
            .request_parse(self.buffer().text().to_string(), language);
    }

    pub(super) fn highlight_snapshot(&self) -> &HighlightSnapshot {
        self.highlighting_service.snapshot()
    }

    pub(super) fn initialize_from_path(&mut self, initial_path: Option<PathBuf>) {
        let Some(path) = initial_path else {
            return;
        };
        let path = path.canonicalize().unwrap_or(path);

        if path.is_dir() {
            self.workspace_path = Some(path.clone());
            self.file_tree = Some(FileTree::new(&path));
            self.editor_config.add_recent(path.clone(), true);
            self.workspace_watcher = Self::start_workspace_watcher(&path, self.event_tx.clone());
            self.open_workspace_last_file(&path);
            self.reveal_active_in_tree = true;
            return;
        }

        if path.is_file()
            && let Ok(buffer) = self.rt.block_on(Buffer::from_file(&path))
        {
            self.documents = vec![OpenDocument::from_buffer(buffer)];
            self.active_document = 0;
            self.editor_config.add_recent(path, false);
        }
    }

    fn track_file_open(&mut self, path: &Path) {
        if let Some(workspace_path) = self
            .workspace_path
            .as_ref()
            .filter(|workspace_path| path.starts_with(workspace_path))
        {
            self.editor_config
                .set_workspace_last_file(workspace_path, path);
            self.reveal_active_in_tree = true;
            return;
        }
        self.editor_config.add_recent(path.to_path_buf(), false);
    }

    fn open_workspace_last_file(&mut self, workspace_path: &Path) {
        let Some(path) = self.editor_config.workspace_last_file(workspace_path) else {
            return;
        };
        if path.is_file()
            && let Ok(buffer) = self.rt.block_on(Buffer::from_file(&path))
        {
            self.documents = vec![OpenDocument::from_buffer(buffer)];
            self.active_document = 0;
            self.refresh_language_intelligence();
        }
    }

    pub(super) fn update_window_title(&self, ctx: &egui::Context) {
        let active_document = self.active_document();
        let title = if let Some(path) = active_document.buffer.path() {
            let dirty_prefix = if active_document.document_dirty {
                "* "
            } else {
                ""
            };
            format!("lux - {}{}", dirty_prefix, path.display())
        } else if active_document.document_dirty {
            "lux - * Untitled".to_string()
        } else {
            "lux".to_string()
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    pub(super) fn switch_to_document(&mut self, index: usize, ctx: &egui::Context) {
        if index >= self.documents.len() {
            return;
        }
        self.active_document = index;
        self.touch_caret_blink();
        self.update_window_title(ctx);
        self.refresh_language_intelligence();
        self.reveal_active_in_tree = true;
    }

    pub(super) fn close_document(&mut self, index: usize, ctx: &egui::Context) {
        if index >= self.documents.len() {
            return;
        }

        if self.documents.len() == 1 {
            self.documents[0] = OpenDocument::new_empty();
            self.active_document = 0;
            self.touch_caret_blink();
            self.update_window_title(ctx);
            self.refresh_language_intelligence();
            return;
        }

        self.documents.remove(index);
        if self.active_document >= self.documents.len() {
            self.active_document = self.documents.len().saturating_sub(1);
        } else if index < self.active_document {
            self.active_document = self.active_document.saturating_sub(1);
        }
        self.touch_caret_blink();
        self.update_window_title(ctx);
        self.refresh_language_intelligence();
        self.reveal_active_in_tree = true;
    }

    fn should_reuse_active_document_slot(&self) -> bool {
        if self.documents.len() != 1 || self.active_document != 0 {
            return false;
        }
        let active_document = self.active_document();
        active_document.buffer.path().is_none()
            && !active_document.document_dirty
            && active_document.buffer.text().len_chars() == 0
    }
}
