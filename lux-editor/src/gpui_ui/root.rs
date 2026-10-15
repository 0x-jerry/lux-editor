//! Root view: owns the application state and drives the logic pass.
//!
//! During the migration the domain/action layer still lives in
//! [`crate::app::App`]; this view embeds it and runs the pre-render logic pass
//! on a frame tick, then renders the gpui-kit shell. Views are moved out of
//! `App` into gpui-kit elements step by step.

use super::theme::{Palette, apply};
use crate::app::App;
use crate::settings::Config;
use crate::theme::{StartupFont, ThemeChoice};
use eframe::egui;
use gpui_kit::*;
use std::time::Duration;

/// The editor repaints at this cadence while it is the running loop; it drives
/// the logic pass (caret blink, debounced work, background-event drain).
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

pub struct Root {
    app: App,
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

        let font_loader = StartupFont::spawn(config.settings.font.family.clone());
        let app = App::new(egui::Context::default(), font_loader, config);

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(FRAME_INTERVAL).await;
                let alive = this.update(cx, |root, cx| {
                    root.app.ctx().frame_logic();
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();

        Self { app }
    }
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::get(cx);
        let title = self.app.tabs.tabs[self.app.tabs.active_tab].meta().title;
        let sidebar = self
            .app
            .workspace
            .path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.window_bg)
            .text_color(palette.text)
            .child(
                div()
                    .h(px(32.0))
                    .flex()
                    .items_center()
                    .px_3()
                    .bg(palette.panel_bg)
                    .border_b_1()
                    .border_color(palette.border)
                    .child(title),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .min_h_0()
                    .child(
                        div()
                            .w(px(220.0))
                            .h_full()
                            .bg(palette.panel_bg)
                            .border_r_1()
                            .border_color(palette.border)
                            .p_2()
                            .text_sm()
                            .text_color(palette.weak)
                            .child(sidebar),
                    )
                    .child(div().flex_1().p_4()),
            )
            .child(
                div()
                    .h(px(24.0))
                    .bg(palette.panel_bg)
                    .border_t_1()
                    .border_color(palette.border),
            )
    }
}
