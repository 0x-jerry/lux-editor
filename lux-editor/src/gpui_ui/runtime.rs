//! Background runtime and the event channel the gpui app drains.
//!
//! Domain code reports through [`CustomEvent`] over a plain channel and wakes
//! the loop through the [`Wake`] handle; a foreground task bridges the wake
//! channel into a gpui notification.

use crate::events::CustomEvent;
use crate::wake::Wake;
use std::future::Future;
use std::sync::mpsc::{Receiver, Sender};

pub struct Runtime {
    pub rt: tokio::runtime::Runtime,
    pub event_tx: Sender<CustomEvent>,
    pub event_rx: Receiver<CustomEvent>,
    pub wake: Wake,
}

impl Runtime {
    /// Returns the runtime plus the receiver side of the wake channel, which
    /// the view owns as the pump it awaits.
    pub fn new() -> (Self, tokio::sync::mpsc::UnboundedReceiver<()>) {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let (wake_tx, wake_rx) = tokio::sync::mpsc::unbounded_channel();
        let runtime = Self {
            rt,
            event_tx,
            event_rx,
            wake: Wake::channel(wake_tx),
        };
        (runtime, wake_rx)
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.rt.spawn(future);
    }
}
