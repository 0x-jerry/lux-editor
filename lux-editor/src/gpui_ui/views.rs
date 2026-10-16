//! Read-only file tabs: images drawn from disk, and the notice shown for files
//! that are neither text nor images.

use super::document::is_binary;
use super::theme::Palette;
use gpui_kit::*;
use std::path::Path;

/// Why a file has no editor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Notice {
    /// The file contains a NUL byte, so it is not text.
    Binary,
    /// The path is gone or unreadable.
    Missing,
}

/// What a file turned out to be, and therefore how it should be shown.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Content {
    /// Its decoded text.
    Text(String),
    /// An image the viewer can decode.
    Image,
    /// Neither text nor an image.
    Binary,
    /// Unreadable: gone, or not permitted.
    Missing,
}

/// Classify `path` by reading it. Images come first because their bytes look
/// binary, then the NUL sniff, then UTF-8.
pub fn classify(path: &Path) -> Content {
    let Ok(bytes) = std::fs::read(path) else {
        return Content::Missing;
    };
    if is_image(path) {
        return Content::Image;
    }
    if is_binary(&bytes) {
        return Content::Binary;
    }
    match String::from_utf8(bytes) {
        Ok(text) => Content::Text(text),
        Err(_) => Content::Binary,
    }
}

/// Whether `path` names an image the viewer can decode.
///
/// SVG is deliberately excluded: it is valid UTF-8, and editing it as text is
/// more useful than rendering it.
pub fn is_image(path: &Path) -> bool {
    const EXTENSIONS: [&str; 8] = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tiff"];
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .is_some_and(|extension| EXTENSIONS.contains(&extension.as_str()))
}

/// The image tab body: the decoded file, scaled down to fit the pane.
pub fn image(path: &Path) -> AnyElement {
    div()
        .flex_1()
        .min_h_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            img(path.to_path_buf())
                .max_w_full()
                .max_h_full()
                .object_fit(ObjectFit::Contain),
        )
        .into_any_element()
}

/// The notice tab body: what the file is, and why it has no editor.
pub fn notice(path: &Path, kind: Notice, cx: &App) -> AnyElement {
    let palette = Palette::get(cx);
    let detail = match kind {
        Notice::Binary => "This file is binary; Lux opens text, images and nothing else.",
        Notice::Missing => "The file was moved or deleted after it was opened.",
    };
    div()
        .flex_1()
        .min_h_0()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .text_color(palette.weak)
        .child(
            div()
                .text_lg()
                .text_color(palette.text)
                .child(match kind {
                    Notice::Binary => "Binary file",
                    Notice::Missing => "File not found",
                }),
        )
        .child(div().max_w(px(480.0)).child(path.display().to_string()))
        .child(div().text_sm().child(detail))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // Deliberately not `use super::*`: gpui exports its own `test` attribute,
    // which would shadow the built-in one below.
    use super::{Content, classify};
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    /// Write `bytes` to `name` in a fresh directory, which the returned guard
    /// deletes when the test ends.
    fn write(name: &str, bytes: &[u8]) -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("temporary directory");
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).expect("write fixture");
        (dir, path)
    }

    #[test]
    fn reads_text_files() {
        let (_dir, path) = write("notes.txt", b"hello\n");
        assert_eq!(classify(&path), Content::Text("hello\n".to_string()));
    }

    #[test]
    fn missing_files_are_reported_as_missing() {
        assert_eq!(classify(Path::new("no/such/file.txt")), Content::Missing);
    }

    #[test]
    fn nul_bytes_make_a_file_binary() {
        let (_dir, path) = write("blob.bin", b"\x00\x01\x02");
        assert_eq!(classify(&path), Content::Binary);
    }

    #[test]
    fn images_win_over_the_binary_sniff() {
        // A PNG header is enough: the extension decides, not the decode.
        let (_dir, path) = write("pixel.png", b"\x89PNG\r\n\x1a\n\x00\x00");
        assert_eq!(classify(&path), Content::Image);
    }

    #[test]
    fn invalid_utf8_is_binary() {
        let (_dir, path) = write("latin1.txt", b"caf\xe9");
        assert_eq!(classify(&path), Content::Binary);
    }

    #[test]
    fn svg_stays_text() {
        let (_dir, path) = write("icon.svg", b"<svg/>");
        assert_eq!(classify(&path), Content::Text("<svg/>".to_string()));
    }
}
