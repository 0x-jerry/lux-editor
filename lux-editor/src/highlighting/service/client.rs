use ropey::Rope;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use super::snapshot::HighlightSnapshot;
use super::worker::worker_loop;
use super::LanguageKind;
use crate::theme::SyntaxColors;
use std::sync::Arc;

pub(super) enum WorkerRequest {
    Parse {
        version: u64,
        /// Shared rope: cloning is O(1), so edits hand the worker the whole
        /// text without copying it on the UI thread.
        text: Rope,
        language: LanguageKind,
        syntax: Arc<SyntaxColors>,
    },
    Shutdown,
}

pub(super) struct WorkerResponse {
    pub version: u64,
    pub snapshot: HighlightSnapshot,
}

pub struct HighlightingService {
    request_tx: Sender<WorkerRequest>,
    response_rx: Receiver<WorkerResponse>,
    worker: Option<JoinHandle<()>>,
    latest: HighlightSnapshot,
    next_version: u64,
    syntax: Arc<SyntaxColors>,
}

impl HighlightingService {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel();
        let (response_tx, response_rx) = mpsc::channel();
        let worker = thread::spawn(move || worker_loop(request_rx, response_tx));
        Self {
            request_tx,
            response_rx,
            worker: Some(worker),
            latest: HighlightSnapshot::default(),
            next_version: 0,
            syntax: crate::theme::syntax_colors(crate::theme::ThemeChoice::Dark),
        }
    }

    /// Handle of the palette the last parse used; `Arc::ptr_eq` against a
    /// fresh [`crate::theme::syntax_colors`] tells whether re-highlighting
    /// is needed without comparing palette contents.
    pub fn syntax(&self) -> &Arc<SyntaxColors> {
        &self.syntax
    }

    pub fn set_syntax(&mut self, syntax: Arc<SyntaxColors>) {
        self.syntax = syntax;
    }

    pub fn request_parse(&mut self, text: Rope, language: LanguageKind) {
        self.next_version += 1;
        self.request_tx
            .send(WorkerRequest::Parse {
                version: self.next_version,
                text,
                language,
                syntax: Arc::clone(&self.syntax),
            })
            .ok();
    }

    pub fn update(&mut self) {
        while let Ok(response) = self.response_rx.try_recv() {
            if response.version >= self.latest.version {
                self.latest = response.snapshot;
            }
        }
    }

    pub fn snapshot(&self) -> &HighlightSnapshot {
        &self.latest
    }
}

impl Drop for HighlightingService {
    fn drop(&mut self) {
        self.request_tx.send(WorkerRequest::Shutdown).ok();
        if let Some(worker) = self.worker.take() {
            worker.join().ok();
        }
    }
}
