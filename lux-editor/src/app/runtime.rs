//! The async runtime and event bus that background workers report through.
//!
//! Owned by [`App`](super::state::App) and shared, read-only, with every
//! action via [`Ctx`](super::context::Ctx). A task is spawned on the tokio
//! runtime, sends its outcome through `event_tx` and wakes the egui loop via
//! `ctx` so the idle app still renders it.

use crate::events::CustomEvent;
use eframe::egui;
use std::future::Future;
use std::sync::mpsc::{Receiver, Sender};

/// Async runtime and the channel the app's background workers report through.
/// `ctx` is the wake handle for the egui loop: producers request a repaint
/// after sending so the idle app still renders the events they push.
pub(crate) struct Runtime {
    pub(crate) rt: tokio::runtime::Runtime,
    pub(crate) event_tx: Sender<CustomEvent>,
    pub(crate) event_rx: Receiver<CustomEvent>,
    pub(crate) ctx: egui::Context,
}

impl Runtime {
    /// Spawn a future on the background runtime.
    pub(crate) fn spawn<F>(&self, future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.rt.spawn(future);
    }

    /// Spawn a blocking job (file IO, formatter runs) on the background
    /// runtime's blocking pool.
    pub(crate) fn spawn_blocking<F, R>(&self, job: F)
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.rt.spawn_blocking(job);
    }
}
