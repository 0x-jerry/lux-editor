//! gpui-kit application tree: the whole presentation layer.

mod actions;
mod document;
mod language;
mod markdown;
mod palette;
mod root;
mod runtime;
mod settings_view;
mod state;
mod tabs;
mod theme;
mod views;

use crate::settings::Config;
use gpui_kit::assets::Assets;
use gpui_kit::component::TitleBar;
use gpui_kit::*;

pub fn run() {
    crate::logging::init(&Config::configured_log_level());

    application().with_assets(Assets).run(|cx| {
        init(cx);
        actions::install(cx);
        open_window(window_options(cx), cx, |window, cx| {
            cx.new(|cx| root::Root::new(window, cx))
        })
        .expect("failed to open Lux window");
    });
}

/// Window options for the app's custom title bar (undecorated client area with
/// app-owned dragging and window controls).
fn window_options(cx: &mut App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::centered(size(px(1100.), px(760.)), cx)),
        window_min_size: Some(size(px(480.), px(320.))),
        ..TitleBar::window_options()
    }
}
