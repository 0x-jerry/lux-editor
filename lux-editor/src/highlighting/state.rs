//! Highlighting domain: the syntax service and the debounced refresh flags.
//! Deciding *when* to refresh and deriving the palette live in
//! `app::actions::highlighting`, which has the tabs/chrome context the
//! refresh needs; this module only owns the state.

use crate::highlighting::HighlightingService;
use std::time::{Duration, Instant};

pub(crate) struct Highlighting {
    pub(crate) service: HighlightingService,
    pub(crate) dirty: bool,
    pub(crate) deadline: Option<Instant>,
}

impl Default for Highlighting {
    fn default() -> Self {
        Self {
            service: HighlightingService::new(),
            dirty: false,
            deadline: None,
        }
    }
}

impl Highlighting {
    pub(crate) const HIGHLIGHT_DEBOUNCE: Duration = Duration::from_millis(60);

    pub(crate) fn schedule_refresh(&mut self) {
        self.dirty = true;
        self.deadline = Some(Instant::now() + Self::HIGHLIGHT_DEBOUNCE);
    }

    /// Whether a scheduled refresh has reached its deadline and should run.
    /// Drains `dirty`/`deadline` as a side effect, so a true result means the
    /// caller owns the refresh.
    pub(crate) fn take_due_refresh(&mut self) -> bool {
        if self.dirty
            && self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.dirty = false;
            self.deadline = None;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlighting::LanguageKind;
    use ropey::Rope;

    #[test]
    fn service_round_trips_rope_snapshot() {
        let mut service = HighlightingService::new();
        service.request_parse(Rope::from_str("fn main() {}\n"), LanguageKind::Rust);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while service.snapshot().version == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
            service.update();
        }
        assert_eq!(service.snapshot().version, 1);
        assert!(!service.snapshot().line_tokens[0].is_empty());
    }
}
