//! Cross-thread wake handle. Background workers and file watchers run off the
//! UI thread and request a repaint through this instead of holding a
//! rendering-layer handle directly, so the same domain code drives both the
//! legacy egui loop and the gpui loop.

use eframe::egui;

/// Wakes the UI thread: either the egui context or a gpui repaint channel.
#[derive(Clone)]
pub enum Wake {
    Egui(egui::Context),
    Channel(tokio::sync::mpsc::UnboundedSender<()>),
}

impl Wake {
    pub fn egui(ctx: egui::Context) -> Self {
        Self::Egui(ctx)
    }

    pub fn channel(tx: tokio::sync::mpsc::UnboundedSender<()>) -> Self {
        Self::Channel(tx)
    }

    pub fn request_repaint(&self) {
        match self {
            Self::Egui(ctx) => ctx.request_repaint(),
            Self::Channel(tx) => {
                let _ = tx.send(());
            }
        }
    }
}

impl From<egui::Context> for Wake {
    fn from(ctx: egui::Context) -> Self {
        Self::Egui(ctx)
    }
}
