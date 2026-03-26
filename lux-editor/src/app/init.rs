use super::{App, ShellView};
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
            buffer: Buffer::new(),
            workspace_path: None,
            file_tree: None,
            workspace_watcher: None,
            settings_watcher: None,
            config_draft: editor_config.settings.clone(),
            editor_config,
            config_status: None,
            config_autosave_deadline: None,
            document_dirty: false,
            document_status: None,
            highlighting_service: crate::language::HighlightingService::new(),
            needs_style_refresh: true,
            reveal_active_in_tree: false,
            shell_view: ShellView::Editor,
            command_panel: Default::default(),
            caret_state: Default::default(),
            edit_history: Default::default(),
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
        if let Ok(buffer) = self.rt.block_on(Buffer::from_file(&path)) {
            self.buffer = buffer;
            self.reset_editor_state();
            self.reset_document_state();
            self.update_window_title(ctx);
            self.track_file_open(&path);
            self.refresh_language_intelligence();
        }
    }

    pub(super) fn save_current_buffer(&mut self, ctx: &egui::Context) -> bool {
        if self.buffer.path().is_none() {
            if let Some(path) = rfd::FileDialog::new().save_file() {
                self.buffer.set_path(&path);
            } else {
                self.document_status = Some("Save cancelled".to_string());
                return false;
            }
        }

        if self.rt.block_on(self.buffer.save()).is_ok() {
            let saved_path = self.buffer.path().cloned().unwrap();
            self.document_dirty = false;
            self.document_status = Some(format!("Saved {}", saved_path.display()));
            self.track_file_open(&saved_path);
            self.update_window_title(ctx);
            self.on_file_change();
            true
        } else {
            self.document_status = Some("Failed to save file".to_string());
            false
        }
    }

    pub(super) fn refresh_language_intelligence(&mut self) {
        self.highlighting_service.set_theme(HighlightThemeConfig {
            theme_name: self.editor_config.settings.theme.syntax_theme.clone(),
            theme_path: self.editor_config.settings.theme.theme_path.clone(),
        });
        let language = LanguageKind::from_path(self.buffer.path().map(|v| &**v));
        self.highlighting_service
            .request_parse(self.buffer.text().to_string(), language);
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
            self.buffer = buffer;
            self.reset_editor_state();
            self.reset_document_state();
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
            self.buffer = buffer;
            self.reset_editor_state();
            self.reset_document_state();
            self.refresh_language_intelligence();
        }
    }

    pub(super) fn reset_document_state(&mut self) {
        self.document_dirty = false;
        self.document_status = None;
    }

    pub(super) fn update_window_title(&self, ctx: &egui::Context) {
        let title = if let Some(path) = self.buffer.path() {
            let dirty_prefix = if self.document_dirty { "* " } else { "" };
            format!("lux - {}{}", dirty_prefix, path.display())
        } else if self.document_dirty {
            "lux - * Untitled".to_string()
        } else {
            "lux".to_string()
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }
}
