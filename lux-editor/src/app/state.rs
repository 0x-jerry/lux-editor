use crate::config::Config;
use crate::events::CustomEvent;
use crate::ui::theme::StartupFont;
use eframe::egui;
use lux_core::Buffer;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

use super::OpenDocument;
use super::chrome::Chrome;
use super::documents::Documents;
use super::highlighting::Highlighting;
use super::settings::SettingsState;
use super::workspace::Workspace;

/// Async runtime and the channel the app's background workers report through.
/// `ctx` is the wake handle for the egui loop: producers request a repaint
/// after sending so the idle app still renders the events they push.
pub(crate) struct Runtime {
    pub(crate) rt: tokio::runtime::Runtime,
    pub(crate) event_tx: Sender<CustomEvent>,
    pub(crate) event_rx: Receiver<CustomEvent>,
    pub(crate) ctx: egui::Context,
}

pub struct App {
    pub(crate) runtime: Runtime,
    pub(crate) documents: Documents,
    pub(crate) workspace: Workspace,
    pub(crate) settings: SettingsState,
    pub(crate) highlighting: Highlighting,
    pub(crate) chrome: Chrome,
    /// CLI path (folder or file) opened after the first frame paints, so
    /// window bring-up never waits on disk work.
    pub(super) pending_init: Option<PathBuf>,
    pub(super) deferred_init_done: bool,
    pub(super) recent_flush_deadline: Option<Instant>,
}

impl App {
    pub fn new(ctx: egui::Context, font_loader: StartupFont) -> Self {
        crate::startup::stage("window backend ready, app ctor");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (event_tx, event_rx) = mpsc::channel();
        let editor_config = Config::load();
        crate::startup::stage("config loaded");
        let mut app = Self {
            runtime: Runtime {
                rt,
                event_tx,
                event_rx,
                ctx,
            },
            documents: Documents::with_empty_document(),
            workspace: Workspace::default(),
            settings: SettingsState {
                editor_config,
                ..Default::default()
            },
            highlighting: Highlighting::default(),
            chrome: Chrome {
                needs_style_refresh: true,
                startup_font: Some(font_loader),
                ..Default::default()
            },
            pending_init: std::env::args().nth(1).map(PathBuf::from),
            deferred_init_done: false,
            recent_flush_deadline: None,
        };
        app.chrome
            .shell
            .sync_config_draft(&app.settings.editor_config.settings);
        crate::startup::stage("app constructed");
        app
    }

    pub(in crate::app) fn active_document(&self) -> &OpenDocument {
        self.documents.active_document()
    }

    pub(in crate::app) fn active_document_mut(&mut self) -> &mut OpenDocument {
        self.documents.active_document_mut()
    }

    pub(in crate::app) fn buffer(&self) -> &Buffer {
        &self.active_document().buffer
    }

    pub(in crate::app) fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.active_document_mut().buffer
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.settings.editor_config.flush_recent();
    }
}
