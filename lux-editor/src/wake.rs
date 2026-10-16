//! Cross-thread wake handle. Background workers and file watchers run off the
//! UI thread and request a repaint through this instead of holding a rendering
//! handle directly.

/// Wakes the UI thread through the runtime's repaint channel.
#[derive(Clone)]
pub struct Wake(tokio::sync::mpsc::UnboundedSender<()>);

impl Wake {
    pub fn channel(tx: tokio::sync::mpsc::UnboundedSender<()>) -> Self {
        Self(tx)
    }

    pub fn request_repaint(&self) {
        let _ = self.0.send(());
    }
}
