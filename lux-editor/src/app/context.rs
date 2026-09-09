//! The per-frame working bundle: every action (`impl Ctx` in `app::actions`)
//! receives a short-lived [`Ctx`] instead of reaching into `App`. Constructing
//! it is the one place that borrows all five domains at once, which keeps the
//! borrow checker happy for the whole action layer.

use super::state::{App, FrameState};
use crate::app::Runtime;
use crate::chrome::Chrome;
use crate::tabs::TabManager;
use crate::highlighting::Highlighting;
use crate::settings::SettingsState;
use crate::workspace::Workspace;
use eframe::egui;

/// Mutable access to every domain for the duration of one action.
pub(crate) struct Ctx<'a> {
    pub(crate) tabs: &'a mut TabManager,
    pub(crate) workspace: &'a mut Workspace,
    pub(crate) settings: &'a mut SettingsState,
    pub(crate) highlighting: &'a mut Highlighting,
    pub(crate) chrome: &'a mut Chrome,
    pub(crate) runtime: &'a Runtime,
    /// App-level plumbing that belongs to no single domain.
    pub(crate) frame: &'a mut FrameState,
}

impl Ctx<'_> {
    /// The wake/repaint handle for the egui loop; also the channel to the
    /// app's rendered chrome (viewport commands, clipboard, input state).
    pub(crate) fn egui_ctx(&self) -> &egui::Context {
        &self.runtime.ctx
    }
}

impl App {
    /// Borrow every domain at once for one action pass.
    pub(crate) fn ctx(&mut self) -> Ctx<'_> {
        Ctx {
            tabs: &mut self.tabs,
            workspace: &mut self.workspace,
            settings: &mut self.settings,
            highlighting: &mut self.highlighting,
            chrome: &mut self.chrome,
            runtime: &self.runtime,
            frame: &mut self.frame,
        }
    }
}
