use crate::config::Config;
use crate::events::CustomEvent;
use lux_core::Buffer;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use super::chrome::Chrome;
use super::documents::Documents;
use super::highlighting::Highlighting;
use super::settings::SettingsState;
use super::workspace::Workspace;
use super::OpenDocument;

/// Async runtime and the channel the app's background workers report through.
pub(crate) struct Runtime {
    pub(crate) rt: tokio::runtime::Runtime,
    pub(crate) event_tx: Sender<CustomEvent>,
    pub(crate) event_rx: Receiver<CustomEvent>,
}

pub struct App {
    pub(crate) runtime: Runtime,
    pub(crate) documents: Documents,
    pub(crate) workspace: Workspace,
    pub(crate) settings: SettingsState,
    pub(crate) highlighting: Highlighting,
    pub(crate) chrome: Chrome,
}

impl App {
    pub fn new() -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (event_tx, event_rx) = mpsc::channel();
        let editor_config = Config::load();
        let mut app = Self {
            runtime: Runtime { rt, event_tx, event_rx },
            documents: Documents::with_empty_document(),
            workspace: Workspace::default(),
            settings: SettingsState {
                editor_config,
                ..Default::default()
            },
            highlighting: Highlighting::default(),
            chrome: Chrome {
                needs_style_refresh: true,
                ..Default::default()
            },
        };
        let initial_path = std::env::args().nth(1).map(PathBuf::from);
        app.initialize_from_path(initial_path);
        app.settings.editor_config.reload_settings();
        app.chrome
            .shell
            .sync_config_draft(&app.settings.editor_config.settings);
        app.restart_settings_watcher();
        app.refresh_language_intelligence();
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