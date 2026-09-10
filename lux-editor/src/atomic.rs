//! Atomic file replacement for everything the editor writes itself.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Distinguishes temp files of two writes to the same path that overlap (a
/// second save while the first is still in flight).
static WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Write `bytes` to `path` so the target is never observable as a partial
/// file: the bytes land in a sibling temp file, are flushed to disk, and only
/// then replace the target by rename. A crash, a full disk or an interrupted
/// write leaves the previous content intact.
pub(crate) fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temp = parent.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "lux".to_string()),
        std::process::id(),
        WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        // A script that was executable has to stay executable.
        if let Ok(metadata) = std::fs::metadata(path) {
            let _ = std::fs::set_permissions(&temp, metadata.permissions());
        }
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::write;

    #[test]
    fn replaces_content_and_keeps_the_file_in_place_on_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        std::fs::write(&path, "old").unwrap();

        write(&path, b"new content").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new content");
        // No temp file is left behind next to the target.
        let leftovers = std::fs::read_dir(dir.path())
            .unwrap()
            .filter(|entry| {
                entry
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".tmp")
            })
            .count();
        assert_eq!(leftovers, 0);

        // A write into a directory that does not exist fails and changes nothing.
        let missing = dir.path().join("nope").join("doc.txt");
        assert!(write(&missing, b"x").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new content");
    }
}
