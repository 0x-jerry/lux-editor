use super::color::parse_rgba;

/// The chrome colors of a theme file, as raw (non-premultiplied) RGBA bytes.
pub struct ChromeColors {
    pub accent: [u8; 4],
    pub text: [u8; 4],
    pub weak: [u8; 4],
    pub border: [u8; 4],
    pub raised: [u8; 4],
    pub hover: [u8; 4],
    pub active: [u8; 4],
    pub panel_bg: [u8; 4],
    pub window_bg: [u8; 4],
    pub extreme_bg: [u8; 4],
    pub faint_bg: [u8; 4],
    pub code_bg: [u8; 4],
    pub selection_bg: [u8; 4],
    pub selection_stroke: [u8; 4],
}

#[derive(serde::Deserialize)]
struct RawTheme {
    #[allow(dead_code)] // "name" is part of the file format for humans/tools
    name: String,
    colors: RawChrome,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawChrome {    accent: String,
    text: String,
    weak: String,
    border: String,
    raised: String,
    hover: String,
    active: String,
    panel_bg: String,
    window_bg: String,
    extreme_bg: String,
    faint_bg: String,
    code_bg: String,
    selection_bg: String,
    selection_stroke: String,
}

impl ChromeColors {
    pub(super) fn parse(json: &str) -> Result<Self, String> {
        let raw: RawTheme = serde_json::from_str(json).map_err(|e| e.to_string())?;
        Ok(Self {
            accent: parse_rgba(&raw.colors.accent)?,
            text: parse_rgba(&raw.colors.text)?,
            weak: parse_rgba(&raw.colors.weak)?,
            border: parse_rgba(&raw.colors.border)?,
            raised: parse_rgba(&raw.colors.raised)?,
            hover: parse_rgba(&raw.colors.hover)?,
            active: parse_rgba(&raw.colors.active)?,
            panel_bg: parse_rgba(&raw.colors.panel_bg)?,
            window_bg: parse_rgba(&raw.colors.window_bg)?,
            extreme_bg: parse_rgba(&raw.colors.extreme_bg)?,
            faint_bg: parse_rgba(&raw.colors.faint_bg)?,
            code_bg: parse_rgba(&raw.colors.code_bg)?,
            selection_bg: parse_rgba(&raw.colors.selection_bg)?,
            selection_stroke: parse_rgba(&raw.colors.selection_stroke)?,
        })
    }
}
