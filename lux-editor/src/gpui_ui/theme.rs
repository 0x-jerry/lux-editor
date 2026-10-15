//! Bridge from Lux's theme JSON (chrome colors) into gpui-kit's color types.
//!
//! The full palette is published as a [`Palette`] global so every view reads
//! the same values without threading them through props. gpui-kit's semantic
//! `Theme` is left at its defaults; Lux owns its own visual language.

use crate::theme::{ChromeColors, ThemeChoice, builtin, resolve};
use gpui_kit::component::theme::Theme;
use gpui_kit::*;

/// Resolved chrome palette, one field per token in `assets/themes/*.json`.
#[derive(Clone, Copy)]
pub struct Palette {
    pub accent: Hsla,
    pub text: Hsla,
    pub weak: Hsla,
    pub border: Hsla,
    pub raised: Hsla,
    pub hover: Hsla,
    pub active: Hsla,
    pub panel_bg: Hsla,
    pub window_bg: Hsla,
    pub extreme_bg: Hsla,
    pub faint_bg: Hsla,
    pub code_bg: Hsla,
    pub selection_bg: Hsla,
    pub selection_stroke: Hsla,
}

impl Global for Palette {}

impl Palette {
    fn from_chrome(colors: &ChromeColors) -> Self {
        Self {
            accent: color(colors.accent),
            text: color(colors.text),
            weak: color(colors.weak),
            border: color(colors.border),
            raised: color(colors.raised),
            hover: color(colors.hover),
            active: color(colors.active),
            panel_bg: color(colors.panel_bg),
            window_bg: color(colors.window_bg),
            extreme_bg: color(colors.extreme_bg),
            faint_bg: color(colors.faint_bg),
            code_bg: color(colors.code_bg),
            selection_bg: color(colors.selection_bg),
            selection_stroke: color(colors.selection_stroke),
        }
    }

    pub fn get(cx: &App) -> Self {
        *cx.global::<Palette>()
    }
}

fn color(value: egui_color::Color32) -> Hsla {
    let [r, g, b, a] = value.to_srgba_unmultiplied();
    rgba(u32::from_be_bytes([r, g, b, a])).into()
}

/// Publish the chrome palette and the configured editor font for `config`.
pub fn apply(cx: &mut App, choice: ThemeChoice, family: &str, size: f32) {
    let resolved = resolve(choice, None);
    cx.set_global(Palette::from_chrome(&builtin(resolved).colors));

    let family = family.to_string();
    Theme::update(cx, |theme| {
        if !family.is_empty() {
            theme.mono_font_family = family.clone().into();
        }
        theme.font_size = px(size);
    });
}

// Lux's theme file stores egui `Color32`; alias the path so this module does
// not re-import the egui crate by its top-level name.
use eframe::egui as egui_color;
