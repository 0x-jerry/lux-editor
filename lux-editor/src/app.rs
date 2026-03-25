use crate::config::Config;
use crate::events::CustomEvent;
use crate::file_tree::FileTree;
use crate::file_watcher;
use crate::language::{HighlightSnapshot, HighlightThemeConfig, HighlightingService, LanguageKind};
use crate::ui;
use eframe::{App as EframeApp, Frame, egui};
use font_kit::family_name::FamilyName;
use font_kit::handle::Handle;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use lux_core::Buffer;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

pub struct App {
    rt: tokio::runtime::Runtime,
    event_tx: Sender<CustomEvent>,
    event_rx: Receiver<CustomEvent>,
    buffer: Buffer,
    workspace_path: Option<PathBuf>,
    file_tree: Option<FileTree>,
    workspace_watcher: Option<RecommendedWatcher>,
    settings_watcher: Option<RecommendedWatcher>,
    editor_config: Config,
    highlighting_service: HighlightingService,
    needs_style_refresh: bool,
}

impl App {
    pub fn new() -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (event_tx, event_rx) = mpsc::channel();
        let mut app = Self {
            rt,
            event_tx,
            event_rx,
            buffer: Buffer::new(),
            workspace_path: None,
            file_tree: None,
            workspace_watcher: None,
            settings_watcher: None,
            editor_config: Config::load(None),
            highlighting_service: HighlightingService::new(),
            needs_style_refresh: true,
        };
        let initial_path = std::env::args().nth(1).map(PathBuf::from);
        app.initialize_from_path(initial_path);
        app.editor_config.reload_settings(app.workspace_path.as_deref());
        app.restart_settings_watcher();
        app.refresh_language_intelligence();
        app
    }

    fn process_pending_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.event_rx.try_recv() {
            self.handle_event(event, ctx);
        }
    }

    fn handle_event(&mut self, event: CustomEvent, ctx: &egui::Context) {
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
        }
    }

    fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }

        let mut changed = false;
        let events = ctx.input(|input| input.events.clone());
        for event in events {
            match event {
                egui::Event::Text(text) => {
                    if !text.starts_with(|c: char| c.is_ascii_control()) {
                        let char_idx = self.buffer.text().len_chars();
                        self.buffer.insert(char_idx, &text);
                        changed = true;
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if modifiers.ctrl || modifiers.command || modifiers.alt {
                        continue;
                    }
                    match key {
                        egui::Key::Enter => {
                            let indentation = Self::indentation_for_newline(&self.buffer);
                            let char_idx = self.buffer.text().len_chars();
                            self.buffer.insert(char_idx, &indentation);
                            changed = true;
                        }
                        egui::Key::Backspace => {
                            let char_idx = self.buffer.text().len_chars();
                            if char_idx > 0 {
                                self.buffer.remove(char_idx - 1..char_idx);
                                changed = true;
                            }
                        }
                        egui::Key::Tab => {
                            let char_idx = self.buffer.text().len_chars();
                            self.buffer.insert(char_idx, "    ");
                            changed = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if changed {
            self.refresh_language_intelligence();
        }
    }

    fn on_file_change(&mut self) {
        if let Some(path) = &self.workspace_path {
            self.file_tree = Some(FileTree::new(path));
        }
    }

    fn on_config_change(&mut self) {
        if self
            .editor_config
            .reload_settings(self.workspace_path.as_deref())
        {
            self.needs_style_refresh = true;
            self.refresh_language_intelligence();
        }
    }

    fn open_folder(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        self.workspace_path = Some(path.clone());
        self.file_tree = Some(FileTree::new(&path));
        self.editor_config.add_recent(path.clone(), true);
        self.workspace_watcher = Self::start_workspace_watcher(&path, self.event_tx.clone());
        self.restart_settings_watcher();
        if self
            .editor_config
            .reload_settings(self.workspace_path.as_deref())
        {
            self.needs_style_refresh = true;
            self.refresh_language_intelligence();
        }
    }

    fn open_file(&mut self, path: PathBuf, ctx: &egui::Context) {
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

    fn refresh_language_intelligence(&mut self) {
        self.highlighting_service.set_theme(HighlightThemeConfig {
            theme_name: self.editor_config.settings.theme.syntax_theme.clone(),
            theme_path: self.editor_config.settings.theme.theme_path.clone(),
        });
        let language = LanguageKind::from_path(self.buffer.path().map(|v| &**v));
        self.highlighting_service
            .request_parse(self.buffer.text().to_string(), language);
    }

    fn highlight_snapshot(&self) -> &HighlightSnapshot {
        self.highlighting_service.snapshot()
    }

    fn initialize_from_path(&mut self, initial_path: Option<PathBuf>) {
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

    fn apply_editor_settings(&mut self, ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        if let Some(custom_font) = Self::load_custom_font(&self.editor_config.settings.font.family)
        {
            fonts.font_data.insert(
                "custom-editor-font".to_string(),
                egui::FontData::from_owned(custom_font).into(),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.insert(0, "custom-editor-font".to_string());
            }
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, "custom-editor-font".to_string());
            }
        }
        ctx.set_fonts(fonts);

        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::monospace(self.editor_config.settings.font.size),
        );
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::proportional(self.editor_config.settings.font.size),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::proportional(self.editor_config.settings.font.size),
        );
        ctx.set_style(style);
    }

    fn load_custom_font(font_family: &str) -> Option<Vec<u8>> {
        let source = SystemSource::new();
        let handle = source
            .select_best_match(&[FamilyName::Title(font_family.to_string())], &Properties::new())
            .ok()?;
        match handle {
            Handle::Path { path, .. } => std::fs::read(path).ok(),
            Handle::Memory { bytes, .. } => Some(bytes.to_vec()),
        }
    }

    fn restart_settings_watcher(&mut self) {
        let watch_roots = Config::settings_watch_roots(self.workspace_path.as_deref());
        self.settings_watcher = Self::start_settings_watcher(&watch_roots, self.event_tx.clone());
    }

    fn start_workspace_watcher(
        workspace_path: &Path,
        event_tx: Sender<CustomEvent>,
    ) -> Option<RecommendedWatcher> {
        if let Ok((watcher, mut rx)) = file_watcher::watch(workspace_path) {
            std::thread::spawn(move || {
                while let Some(result) = rx.blocking_recv() {
                    if result.is_ok() {
                        event_tx.send(CustomEvent::FileChange).ok();
                    }
                }
            });
            Some(watcher)
        } else {
            None
        }
    }

    fn start_settings_watcher(
        watch_roots: &[PathBuf],
        event_tx: Sender<CustomEvent>,
    ) -> Option<RecommendedWatcher> {
        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<notify::Event>| {
                if result.is_ok() {
                    event_tx.send(CustomEvent::ConfigChange).ok();
                }
            },
            notify::Config::default(),
        )
        .ok()?;

        for root in watch_roots {
            if !root.exists() {
                std::fs::create_dir_all(root).ok()?;
            }
            watcher.watch(root, RecursiveMode::NonRecursive).ok()?;
        }

        Some(watcher)
    }

    fn indentation_for_newline(buffer: &Buffer) -> String {
        const INDENT: &str = "    ";

        let total_chars = buffer.text().len_chars();
        if total_chars == 0 {
            return "\n".to_string();
        }

        let line_idx = buffer.text().char_to_line(total_chars.saturating_sub(1));
        let line = buffer.text().line(line_idx).to_string();
        let content = line.trim_end_matches(['\n', '\r']);
        let leading = content
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect::<String>();
        let trimmed = content.trim_end();

        if trimmed.ends_with('{') {
            return format!("\n{}{}", leading, INDENT);
        }

        if trimmed.starts_with('}') {
            let dedented = if leading.ends_with('\t') {
                leading.trim_end_matches('\t').to_string()
            } else if leading.ends_with(INDENT) {
                leading.trim_end_matches(INDENT).to_string()
            } else {
                String::new()
            };
            return format!("\n{}", dedented);
        }

        format!("\n{}", leading)
    }
}

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
