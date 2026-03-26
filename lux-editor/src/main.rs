mod app;
mod config;
mod events;
mod file_tree;
mod file_watcher;
mod language;
mod ui;

use app::App;

pub fn main() {
    env_logger::init();

    let native_options = eframe::NativeOptions::default();
    if let Err(err) = eframe::run_native(
        "Lux Editor",
        native_options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    ) {
        log::error!("failed to run Lux Editor: {err}");
    }
}
