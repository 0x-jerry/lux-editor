//! Root view: owns the application domains and renders the shell frame.

use super::theme::{Palette, apply};
use crate::settings::Config;
use crate::theme::ThemeChoice;
use gpui_kit::*;

pub struct Root {
    pub(crate) config: Config,
}

impl Root {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let config = Config::load();
        apply(
            cx,
            ThemeChoice::from_value(&config.settings.theme.choice),
            &config.settings.font.family,
            config.settings.font.size,
        );
        Self { config }
    }
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::get(cx);
        div()
            .size_full()
            .bg(palette.window_bg)
            .text_color(palette.text)
            .child("Lux Editor")
    }
}
