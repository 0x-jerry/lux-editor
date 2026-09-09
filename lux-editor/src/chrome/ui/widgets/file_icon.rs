//! File-type glyph + brand color (devicons) shared by the file tree and the
//! command panel's recent-file rows.

use eframe::egui::Color32;
use std::path::Path;

/// Root file glyph for types the devicons table does not name.
const GENERIC_FILE_GLYPH: char = '\u{e7b8}';

/// Devicons glyph and brand color for a file row; unknown extensions get
/// [`GENERIC_FILE_GLYPH`] — the crate's own fallback is a literal `'*'`,
/// which the embedded symbols font does not contain.
pub(crate) fn file_type_icon(dark_mode: bool, path: &Path) -> (char, Color32) {
    let theme = if dark_mode {
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
    let color = crate::theme::color::parse_color(icon.color)
        .unwrap_or_else(|_| Color32::from_rgb(0x7e, 0x8e, 0xa8));
    (glyph, color)
}
