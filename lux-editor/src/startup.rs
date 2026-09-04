//! Coarse startup timing, emitted through `log::info!` (visible with
//! `RUST_LOG=info`) and measured from the first instant of `main`.

use std::sync::LazyLock;
use std::time::Instant;

static PROCESS_START: LazyLock<Instant> = LazyLock::new(Instant::now);

pub(crate) fn stage(name: &str) {
    log::info!(
        "[startup] {name}: {:.1}ms",
        PROCESS_START.elapsed().as_secs_f64() * 1000.0
    );
}

/// Logs a milestone at most once; the "once" flag lives at each call site.
macro_rules! stage_once {
    ($name:literal) => {{
        static DONE: ::std::sync::atomic::AtomicBool =
            ::std::sync::atomic::AtomicBool::new(false);
        if !DONE.swap(true, ::std::sync::atomic::Ordering::Relaxed) {
            $crate::startup::stage($name);
        }
    }};
}

pub(crate) use stage_once;
