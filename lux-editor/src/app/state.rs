//! `App`: plain container for the domain states and the runtime that feeds
//! them. Behaviour is layered so this struct stays a dumb holder:
//!
//! - domain structs (`TabManager`, `Workspace`, `SettingsState`,
//!   `Highlighting`, `Chrome`) own their state and pure transitions;
//! - [`Ctx`](super::context::Ctx) is a short-lived bundle of `&mut` borrows
//!   handed to the `app::actions` modules, which implement the cross-domain
//!   behaviour (each action is an `impl Ctx` method);
//! - this module only constructs the domains, runs the eframe loop and
//!   forwards events into a fresh [`Ctx`] each pass.

use crate::app::Runtime;
use crate::chrome::Chrome;
use crate::highlighting::Highlighting;
use crate::settings::{Config, SettingsState};
use crate::tabs::TabManager;
use crate::theme::StartupFont;
use crate::workspace::Workspace;
use eframe::egui;
use std::path::PathBuf;
use std::time::Instant;

pub struct App {
    pub(crate) runtime: Runtime,
    pub(crate) tabs: TabManager,
    pub(crate) workspace: Workspace,
    pub(crate) settings: SettingsState,
    pub(crate) highlighting: Highlighting,
    pub(crate) chrome: Chrome,
    /// App-level plumbing that belongs to no single domain: the CLI path to
    /// open after the first frame, whether that init has run, and the debounce
    /// deadline for the recent-files flush.
    pub(crate) frame: FrameState,
}

/// One-off, app-owned state that crosses every domain but is owned by none of
/// them. Lives beside the domain bundle rather than inside a domain struct.
#[derive(Default)]
pub(crate) struct FrameState {
    /// CLI path (folder or file) opened after the first frame paints, so
    /// window bring-up never waits on disk work.
    pub(crate) pending_init: Option<PathBuf>,
    pub(crate) deferred_init_done: bool,
    /// Debounce deadline for the recent-files flush.
    pub(crate) recent_flush_deadline: Option<Instant>,
    /// A restored session asked for the configuration tab; keyed to the
    /// workspace whose restore should end on it, so a foreign load batch can
    /// neither consume it early nor leave it dangling. Consumed once that
    /// workspace's file batch lands (it activates the remembered file first,
    /// so returning to the editor from configuration finds that file focused).
    pub(crate) pending_configuration_restore: Option<PathBuf>,
    /// Tab id the close prompt asked to save before closing. The tab closes
    /// only when its save lands clean; a failed or superseded save clears it.
    pub(crate) pending_close_after_save: Option<u64>,
    /// A window close is waiting for unsaved work to be resolved: whether the
    /// close is pending, and the tab its prompt is asking about.
    pub(crate) quit_pending: bool,
    pub(crate) quit_prompt_tab: Option<u64>,
}

impl App {
    pub fn new(ctx: egui::Context, font_loader: StartupFont, editor_config: Config) -> Self {
        crate::app::startup::stage("window backend ready, app ctor");
        egui_extras::install_image_loaders(&ctx);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        crate::app::startup::stage("config loaded");
        let mut app = Self {
            // Built before `ctx` moves into the runtime: the highlight worker
            // wakes the UI loop with it when a parse lands.
            highlighting: Highlighting::new(ctx.clone()),
            runtime: Runtime {
                rt,
                event_tx,
                event_rx,
                ctx,
            },
            tabs: TabManager::with_empty_document(),
            workspace: Workspace::default(),
            settings: SettingsState {
                editor_config,
                ..Default::default()
            },
            chrome: Chrome {
                needs_style_refresh: true,
                startup_font: Some(font_loader),
                ..Default::default()
            },
            frame: FrameState {
                pending_init: std::env::args().nth(1).map(PathBuf::from),
                ..Default::default()
            },
        };
        app.chrome
            .shell
            .sync_config_draft(&app.settings.editor_config.settings);
        crate::app::startup::stage("app constructed");
        app
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.settings.editor_config.flush_recent();
    }
}
