use eframe::egui::Color32;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::color::{parse_color, parse_rgba};

/// A parsed theme: chrome colors plus the syntax palette handed to the
/// highlight worker behind an [`Arc`] so palettes compare by identity.
pub struct ThemeFile {
    pub colors: ChromeColors,
    pub syntax: Arc<SyntaxColors>,
}

pub struct ChromeColors {
    pub accent: Color32,
    pub text: Color32,
    pub weak: Color32,
    pub border: Color32,
    pub raised: Color32,
    pub hover: Color32,
    pub active: Color32,
    pub panel_bg: Color32,
    pub window_bg: Color32,
    pub extreme_bg: Color32,
    pub faint_bg: Color32,
    pub code_bg: Color32,
    pub selection_bg: Color32,
    pub selection_stroke: Color32,
}

/// Raw syntax palette: token capture name (tree-sitter highlight names) to
/// color. Names absent from `tokens` render in `foreground`.
#[derive(Clone)]
pub struct SyntaxColors {
    pub foreground: [u8; 4],
    pub background: [u8; 4],
    pub tokens: BTreeMap<String, [u8; 4]>,
}

#[derive(serde::Deserialize)]
struct RawTheme {
    #[allow(dead_code)] // "name" is part of the file format for humans/tools
    name: String,
    colors: RawChrome,
    syntax: RawSyntax,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawChrome {
    accent: String,
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

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSyntax {
    foreground: String,
    background: String,
    tokens: BTreeMap<String, String>,
}

impl ThemeFile {
    pub(super) fn parse(json: &str) -> Result<Self, String> {
        let raw: RawTheme = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let colors = ChromeColors {
            accent: parse_color(&raw.colors.accent)?,
            text: parse_color(&raw.colors.text)?,
            weak: parse_color(&raw.colors.weak)?,
            border: parse_color(&raw.colors.border)?,
            raised: parse_color(&raw.colors.raised)?,
            hover: parse_color(&raw.colors.hover)?,
            active: parse_color(&raw.colors.active)?,
            panel_bg: parse_color(&raw.colors.panel_bg)?,
            window_bg: parse_color(&raw.colors.window_bg)?,
            extreme_bg: parse_color(&raw.colors.extreme_bg)?,
            faint_bg: parse_color(&raw.colors.faint_bg)?,
            code_bg: parse_color(&raw.colors.code_bg)?,
            selection_bg: parse_color(&raw.colors.selection_bg)?,
            selection_stroke: parse_color(&raw.colors.selection_stroke)?,
        };
        let mut tokens = BTreeMap::new();
        for (name, value) in &raw.syntax.tokens {
            tokens.insert(name.clone(), parse_rgba(value)?);
        }
        let syntax = SyntaxColors {
            foreground: parse_rgba(&raw.syntax.foreground)?,
            background: parse_rgba(&raw.syntax.background)?,
            tokens,
        };
        Ok(Self {
            colors,
            syntax: Arc::new(syntax),
        })
    }
}
