use super::{App, ShellView};
use crate::file_tree::FileTree;
use crate::language::{HighlightSnapshot, HighlightThemeConfig, LanguageKind};
use eframe::egui;
use lux_core::Buffer;
use std::path::PathBuf;
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
            highlighting_service: crate::language::HighlightingService::new(),
            needs_style_refresh: true,
            shell_view: ShellView::Editor,
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
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
                "lux - {}",
                path.display()
            )));
            self.editor_config.add_recent(path, false);
            self.refresh_language_intelligence();
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
            return;
        }

        if path.is_file()
            && let Ok(buffer) = self.rt.block_on(Buffer::from_file(&path))
        {
            self.buffer = buffer;
            self.editor_config.add_recent(path, false);
        }
    }
}
