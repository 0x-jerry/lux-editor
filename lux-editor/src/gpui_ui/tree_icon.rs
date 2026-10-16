//! Glyphs for the file tree's rows: devicons file-type glyphs with their brand
//! colors, and the folder glyphs.
//!
//! Every glyph is a Nerd Font private-use codepoint, so a row has to render it
//! with the bundled symbols family; the editor font has no coverage for them.

use crate::theme::color::parse_rgba;
use gpui_kit::*;
use std::path::Path;

/// The family name declared inside
/// `assets/fonts/SymbolsNerdFontMono-Regular.ttf`, which `run` registers with
/// the text system. A mismatch renders every icon as a missing-glyph box.
pub(super) const ICON_FONT_FAMILY: &str = "Symbols Nerd Font Mono";

/// Root file glyph for types the devicons table does not name: the crate's own
/// fallback is a literal `'*'`, which the symbols font does not contain.
const GENERIC_FILE_GLYPH: char = '\u{e7b8}';

/// Color for a file type the devicons table does not name.
const GENERIC_FILE_COLOR: u32 = 0x7e8e_a8;

/// Closed and open folder glyphs. Devicons names file types only, so the
/// folders come from the symbols font's Font Awesome range instead.
const FOLDER_GLYPH: char = '\u{f07b}';
const FOLDER_OPEN_GLYPH: char = '\u{f07c}';

/// The folder glyph for a directory row: open while that row is expanded, which
/// is the tree's only signal that a folder is showing its children.
pub(super) fn folder_glyph(expanded: bool) -> char {
    if expanded {
        FOLDER_OPEN_GLYPH
    } else {
        FOLDER_GLYPH
    }
}

/// File-type glyph and brand color for one file row.
pub(super) fn file_type_icon(path: &Path, dark: bool) -> (char, Hsla) {
    let theme = if dark {
        devicons::Theme::Dark
    } else {
        devicons::Theme::Light
    };
    let icon = devicons::icon_for_file(path, &Some(theme));
    let glyph = if icon.icon == '*' {
        GENERIC_FILE_GLYPH
    } else {
        icon.icon
    };
    let color = parse_rgba(icon.color)
        .map(|bytes| rgba(u32::from_be_bytes(bytes)).into())
        .unwrap_or_else(|_| rgb(GENERIC_FILE_COLOR).into());
    (glyph, color)
}

#[cfg(test)]
mod tests {
    // Deliberately not `use super::*`: gpui exports its own `test` attribute,
    // which would shadow the built-in one below.
    use super::{GENERIC_FILE_COLOR, GENERIC_FILE_GLYPH, file_type_icon};
    use gpui_kit::{Hsla, rgb};
    use std::path::Path;

    #[test]
    fn named_types_get_their_glyph_and_unnamed_ones_the_generic() {
        let (rust, color) = file_type_icon(Path::new("main.rs"), true);
        let fallback: Hsla = rgb(GENERIC_FILE_COLOR).into();
        assert_ne!(rust, GENERIC_FILE_GLYPH);
        assert_ne!(color, fallback);
        // Devicons' own unknown-type entry is the one glyph the font lacks.
        let (unknown, _) = file_type_icon(Path::new("notes.zzz"), true);
        assert_eq!(unknown, GENERIC_FILE_GLYPH);
    }
}
