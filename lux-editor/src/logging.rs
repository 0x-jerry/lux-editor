//! Console + file logger: the configured level, with `RUST_LOG` taking
//! precedence. The file lives in the app data dir and rolls over at
//! [`MAX_LOG_BYTES`], keeping `KEEP_FILES` previous files (`lux.log.1`, …).

use crate::paths::data_dir;
use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};
use std::io::{self, Write};
use std::path::Path;

/// Roll over once the active file passes this size.
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const KEEP_FILES: usize = 1;
const LOG_FILE: &str = "lux.log";

pub(crate) fn init(configured: &str) {
    let mut builder = env_logger::Builder::new();
    if std::env::var_os("RUST_LOG").is_some() {
        builder.parse_default_env();
    } else {
        builder.filter_level(filter(configured));
    }
    // A pipe target writes without ANSI styling (env_logger resolves `Auto` to
    // `Never` for pipes), so the file stays plain at the cost of console color.
    builder.target(env_logger::Target::Pipe(Box::new(LogWriter::new())));
    builder.init();
}

/// True for the log file and its rolled-over backups; the settings watcher
/// watches the same directory and must not read log writes as config changes.
pub(crate) fn is_log_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name == LOG_FILE
        || name
            .strip_prefix(LOG_FILE)
            .is_some_and(|rest| rest.starts_with('.'))
}

fn filter(level: &str) -> log::LevelFilter {
    match level {
        "error" => log::LevelFilter::Error,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        // "warn" and any unknown/hand-edited value fall back to the default.
        _ => log::LevelFilter::Warn,
    }
}

fn open_log_file() -> Option<BasicRollingFileAppender> {
    let path = data_dir().join(LOG_FILE);
    std::fs::create_dir_all(path.parent()?).ok()?;
    let condition = RollingConditionBasic::new().max_size(MAX_LOG_BYTES);
    BasicRollingFileAppender::new(&path, condition, KEEP_FILES).ok()
}

/// Tees env_logger's formatted bytes to stderr and the rolling file.
struct LogWriter {
    console: io::Stderr,
    file: Option<BasicRollingFileAppender>,
}

impl LogWriter {
    fn new() -> Self {
        Self {
            console: io::stderr(),
            file: open_log_file(),
        }
    }
}

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.console.write_all(buf)?;
        if let Some(file) = self.file.as_mut() {
            // Logging must never fail the app; the console copy already landed.
            let _ = file.write_all(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.console.flush()?;
        if let Some(file) = self.file.as_mut() {
            let _ = file.flush();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{is_log_file, filter};
    use log::LevelFilter;
    use std::path::Path;

    #[test]
    fn known_levels_map_and_unknown_falls_back_to_warn() {
        assert_eq!(filter("error"), LevelFilter::Error);
        assert_eq!(filter("warn"), LevelFilter::Warn);
        assert_eq!(filter("info"), LevelFilter::Info);
        assert_eq!(filter("debug"), LevelFilter::Debug);
        assert_eq!(filter("trace"), LevelFilter::Trace);
        assert_eq!(filter("Debug"), LevelFilter::Warn);
        assert_eq!(filter("nonsense"), LevelFilter::Warn);
    }

    #[test]
    fn log_backups_are_recognized_but_other_files_are_not() {
        assert!(is_log_file(Path::new("/home/u/.lux-editor/lux.log")));
        assert!(is_log_file(Path::new("/home/u/.lux-editor/lux.log.1")));
        assert!(!is_log_file(Path::new("/home/u/.lux-editor/config.json")));
        assert!(!is_log_file(Path::new("/home/u/.lux-editor/lux.logger")));
    }
}
