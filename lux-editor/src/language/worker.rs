use std::sync::mpsc::{Receiver, Sender};

use super::engine::Engines;
use super::parse::parse_snapshot;
use super::service::{WorkerRequest, WorkerResponse};

pub(super) fn worker_loop(
    request_rx: Receiver<WorkerRequest>,
    response_tx: Sender<WorkerResponse>,
) {
    let mut engines = Engines::new();

    while let Ok(request) = request_rx.recv() {
        match request {
            WorkerRequest::Shutdown => break,
            WorkerRequest::Parse {
                mut version,
                mut text,
                mut language,
                mut syntax,
            } => {
                // Coalesce queued requests: only the newest state is worth parsing.
                while let Ok(next_request) = request_rx.try_recv() {
                    match next_request {
                        WorkerRequest::Shutdown => return,
                        WorkerRequest::Parse {
                            version: next_version,
                            text: next_text,
                            language: next_language,
                            syntax: next_syntax,
                        } => {
                            version = next_version;
                            text = next_text;
                            language = next_language;
                            syntax = next_syntax;
                        }
                    }
                }

                let snapshot = parse_snapshot(&mut engines, &syntax, &text, language, version);
                response_tx.send(WorkerResponse { version, snapshot }).ok();
            }
        }
    }
}
