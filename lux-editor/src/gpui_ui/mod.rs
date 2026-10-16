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
mod tree_icon;
mod views;

use crate::settings::Config;
use gpui_kit::assets::Assets;
use gpui_kit::component::TitleBar;
use gpui_kit::*;

pub fn run() {
    crate::logging::init(&Config::configured_log_level());

    application().with_assets(Assets).run(|cx| {
        init(cx);
        register_icon_font(cx);
        actions::install(cx);
        open_window(window_options(cx), cx, |window, cx| {
            cx.new(|cx| root::Root::new(window, cx))
        })
        .expect("failed to open Lux window");
    });
}

/// Register the symbols font that carries the sidebar's file-type glyphs. They
/// live in the Nerd Font private-use range, which no system UI font covers, so
/// without this every icon would draw as a missing-glyph box.
fn register_icon_font(cx: &mut App) {
    let font = include_bytes!("../../assets/fonts/SymbolsNerdFontMono-Regular.ttf");
    if let Err(err) = cx.text_system().add_fonts(vec![font.as_slice().into()]) {
        log::warn!("could not register the symbols font: {err}");
    }
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
