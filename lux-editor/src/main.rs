mod atomic;
mod events;
mod formatter;
mod gpui_ui;
mod logging;
mod paths;
mod settings;
mod theme;
mod wake;
mod workspace;

pub fn main() {
    gpui_ui::run();
}
