mod app;
mod atomic;
mod chrome;
mod component;
mod document;
mod events;
mod gpui_ui;
mod highlighting;
mod logging;
mod native;
mod paths;
mod settings;
mod tabs;
mod theme;
mod workspace;

pub fn main() {
    gpui_ui::run();
}
