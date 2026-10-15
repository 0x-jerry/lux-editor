//! gpui-kit application tree. Replaces the eframe/egui presentation layer;
//! the legacy `app`/`chrome`/`tabs`/`document::ui` modules stay compiled during
//! the migration and are removed once every view has been ported.

mod document;
mod language;
mod root;
mod runtime;
mod state;
mod tabs;
mod theme;

use crate::settings::Config;
use gpui_kit::assets::Assets;
use gpui_kit::{AppContext as _, WindowOptions, application, init, open_window};

pub fn run() {
    crate::logging::init(&Config::configured_log_level());

    application().with_assets(Assets).run(|cx| {
        init(cx);
        open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| root::Root::new(window, cx))
        })
        .expect("failed to open Lux window");
    });
}
