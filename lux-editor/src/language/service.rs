use ropey::Rope;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use super::snapshot::HighlightSnapshot;
use super::worker::worker_loop;
use crate::language::{LanguageKind, SyntaxTheme};

pub(super) enum WorkerRequest {
    Parse {
        version: u64,
        /// Shared rope: cloning is O(1), so edits hand the worker the whole
        /// text without copying it on the UI thread.
        text: Rope,
        language: LanguageKind,
        theme: SyntaxTheme,
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
    theme: SyntaxTheme,
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
            theme: SyntaxTheme::Dark,
        }
    }

    pub fn theme(&self) -> SyntaxTheme {
        self.theme
    }

    pub fn set_theme(&mut self, theme: SyntaxTheme) {
        self.theme = theme;
    }

    pub fn request_parse(&mut self, text: Rope, language: LanguageKind) {
        self.next_version += 1;
        self.request_tx
            .send(WorkerRequest::Parse {
                version: self.next_version,
                text,
                language,
                theme: self.theme,
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
