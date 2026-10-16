//! Root view: owns the application state, drains the background event channel
//! and renders the shell (title bar with the application menu, sidebar file
//! tree, tab strip, editor and status bar).
//!
//! The behaviour is split by domain into the sibling modules: [`workspace`]
//! (folder, tree and session), [`documents`] (opening and closing tabs),
//! [`saving`] (writing and formatting buffers), [`settings`] (the settings tab,
//! the theme and the OS appearance), [`chrome`] (what the window draws) and
//! [`shell`] (the layout and the action wiring).

mod chrome;
mod documents;
mod saving;
mod settings;
mod shell;
mod workspace;

use super::palette::{CommandPalette, PaletteEvent};
use super::runtime::Runtime;
use super::settings_view::SettingsView;
use super::state::{LuxState, TreeView};
use super::tabs::Tabs;
use super::theme::apply;
use crate::events::{CustomEvent, WorkspaceEvent};
use crate::settings::Config;
use crate::theme::ThemeChoice;
use crate::workspace::Workspace;
use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::*;
use std::path::PathBuf;

/// Housekeeping tick: recent-items flush debounce, session snapshot. The editor
/// owns its own caret blink, so this stays coarse.
const TICK: std::time::Duration = std::time::Duration::from_millis(500);

pub struct Root {
    state: LuxState,
    menu_bar: Entity<AppMenuBar>,
    /// The command palette, rendered as an overlay while `palette_open`.
    palette: Entity<CommandPalette>,
    _palette_events: Subscription,
    /// The settings page; the tab that shows it holds no state of its own.
    settings_view: Entity<SettingsView>,
    _settings_events: Subscription,
    palette_open: bool,
    sidebar_visible: bool,
    /// A recent item the palette confirmed: opening it needs a window, so the
    /// next render performs it.
    pending_open: Option<PathBuf>,
    /// The settings tab changed the configuration; the write is pending.
    settings_dirty: bool,
    /// The OS appearance, which the `auto` theme choice follows.
    system_is_light: bool,
    focus_handle: FocusHandle,
    /// Last title pushed to the OS, so render does not churn the window.
    last_title: Option<String>,
}

impl Root {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = Config::load();
        let system_is_light = settings::is_light(window.appearance());
        apply(
            cx,
            ThemeChoice::from_value(&config.settings.theme.choice),
            &config.settings.font.family,
            config.settings.font.size,
            system_is_light,
        );

        let (runtime, mut wake_rx) = Runtime::new();
        let settings_watcher = crate::settings::watch(runtime.event_tx.clone(), runtime.wake.clone());
        let menu_bar = AppMenuBar::new(cx);
        let palette = cx.new(|cx| CommandPalette::new(window, cx));
        let settings_view = cx.new(|cx| SettingsView::new(&config.settings, window, cx));
        let _palette_events = cx.subscribe(&palette, |root, _palette, event: &PaletteEvent, cx| {
            root.palette_open = false;
            if let PaletteEvent::Open(path) = event {
                // Opening needs a window, which the event handler has none of;
                // the next render picks this up.
                root.pending_open = Some(path.clone());
            }
            cx.notify();
        });
        let _settings_events = cx.subscribe(&settings_view, Self::on_settings_event);

        let mut root = Self {
            state: LuxState {
                runtime,
                config,
                workspace: Workspace::default(),
                tabs: Tabs::default(),
                tree: TreeView::default(),
                _settings_watcher: settings_watcher,
            },
            menu_bar,
            palette,
            _palette_events,
            settings_view,
            _settings_events,
            palette_open: false,
            sidebar_visible: true,
            pending_open: None,
            settings_dirty: false,
            system_is_light,
            focus_handle: cx.focus_handle(),
            last_title: None,
        };

        // The window's own close button goes through the same gate as Quit. The
        // platform keeps a single should-close callback, so this registration is
        // the window's only one.
        window.on_window_should_close(cx, {
            let this = cx.weak_entity();
            move |window, cx| {
                this.update(cx, |root, cx| root.confirm_all_dirty(window, cx))
                    .unwrap_or(true)
            }
        });

        cx.spawn(async move |this, cx| {
            while wake_rx.recv().await.is_some() {
                let alive = this.update(cx, |root, cx| {
                    root.pump(cx);
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                if this.update(cx, |root, _cx| root.tick()).is_err() {
                    break;
                }
            }
        })
        .detach();

        if let Some(path) = std::env::args().nth(1) {
            root.open_path(PathBuf::from(path), window, cx);
        }

        root.sync_window_title(window);
        root
    }

    /// Background events: a workspace file change rescans the loaded tree; a
    /// configuration change reloads the settings.
    fn pump(&mut self, cx: &mut Context<Self>) {
        while let Ok(event) = self.state.runtime.event_rx.try_recv() {
            match event {
                CustomEvent::Workspace(WorkspaceEvent::FileChange) => {
                    if let Some(tree) = self.state.workspace.file_tree.as_mut() {
                        tree.refresh();
                    }
                    cx.notify();
                }
                CustomEvent::App(crate::events::AppEvent::ConfigChange) => {
                    if self.state.config.reload_settings() {
                        let settings = self.state.config.settings.clone();
                        self.refresh_theme(cx);
                        self.settings_view
                            .update(cx, |view, cx| view.sync(&settings, cx));
                        cx.notify();
                    }
                }
            }
        }
    }

    fn tick(&mut self) {
        self.sync_workspace_session();
        self.state.config.flush_recent();
        self.save_settings();
    }

    /// Show the command palette with the query box focused.
    pub(super) fn open_palette(&mut self, cx: &mut Context<Self>) {
        let recents = self.state.config.recent_items.clone();
        self.palette.update(cx, |palette, cx| {
            palette.set_recents(recents, cx);
            palette.request_focus(cx);
        });
        self.palette_open = true;
        cx.notify();
    }

    /// Hide the palette. Every action it can dispatch does this first, so a
    /// dispatched command never leaves the overlay behind.
    pub(super) fn dismiss_palette(&mut self, cx: &mut Context<Self>) {
        if self.palette_open {
            self.palette_open = false;
            cx.notify();
        }
    }
}
