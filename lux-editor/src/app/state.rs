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
use crate::theme::{self, CustomFont, StartupFont, ThemeChoice};
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
    /// CLI path (folder or file) opened on the first logic pass, before the
    /// first frame paints, so the welcome page never flashes over it. A file's
    /// bytes are read on the background runtime; opening a folder also does its
    /// tree/watcher setup there.
    pub(crate) pending_init: Option<PathBuf>,
    /// The first link's settings watcher has been started. It waits for a
    /// painted window because `notify` setup and settings-dir creation are
    /// disk work that must not delay the first frame.
    pub(crate) settings_watcher_started: bool,
    /// The startup CLI target's initial read is still in flight. The welcome
    /// page stays hidden until it lands; set only by `initialize_from_path`, so
    /// opens made later keep today's behavior.
    pub(crate) startup_open_pending: bool,
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
            highlighting: Highlighting::new(ctx.clone().into()),
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
        // `Context::set_fonts` only takes effect at the start of the *next*
        // pass, and `frame_logic` applies the style at the end of the first
        // pass. The shell can render the file tree — whose icons live in the
        // named `devicons` family — on that first frame when a CLI workspace
        // opens eagerly, so install the base fonts here, before the loop, to
        // bind the family in time. `apply_style` re-applies once the configured
        // editor font lands.
        let resolved = theme::resolve(
            ThemeChoice::from_value(&app.settings.editor_config.settings.theme.choice),
            app.runtime.ctx.system_theme(),
        );
        theme::apply_editor_settings(
            &app.runtime.ctx,
            resolved,
            &app.settings.editor_config.settings,
            CustomFont::Pending,
        );
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

#[cfg(test)]
mod tests {
    use super::App;
    use crate::settings::Config;
    use crate::theme::StartupFont;
    use eframe::egui;

    /// `set_fonts` only takes effect at the start of the *next* pass, and the
    /// file tree — whose icons live in the named `devicons` family — can render
    /// on the first frame of an eagerly opened CLI workspace. `App::new` must
    /// therefore bind the family before the loop, or that first frame panics; a
    /// regression here is a hard crash, not a visual glitch.
    #[test]
    fn app_new_binds_devicons_before_the_first_frame() {
        let ctx = egui::Context::default();
        let _app = App::new(
            ctx.clone(),
            StartupFont::spawn(String::new()),
            Config::default(),
        );
        let mut bound = false;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                ..Default::default()
            },
            |ui| {
                bound = ui.ctx().fonts(|fonts| {
                    fonts
                        .families()
                        .contains(&egui::FontFamily::Name("devicons".into()))
                });
            },
        );
        // The frame's font-atlas upload is not consumed in a headless test.
        output.textures_delta.clear();
        assert!(bound, "the devicons family must be bound on the first pass");
    }
}
