mod app;
mod component;
mod documents;
mod events;
mod workspace;
mod highlighting;
mod native;
mod settings;
mod theme;
mod chrome;

use app::App;
use settings::Config;
use eframe::egui;

pub fn main() {
    env_logger::init();
    app::startup::stage("process start");

    // Start resolving/reading the configured editor font before the window
    // machinery spins up; the app folds the bytes in when they land.
    let font_loader = theme::StartupFont::spawn(Config::load_settings().font.family);

    // Platform adapter for the app-rendered title bar:
    // - macOS keeps the native title bar transparent (traffic lights stay
    //   native) and renders content underneath it.
    // - Other platforms drop OS decorations entirely; the widgets then render
    //   window controls and a resize handle.
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 760.0])
        .with_min_inner_size([600.0, 400.0]);
    #[cfg(target_os = "macos")]
    {
        viewport = viewport
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_fullsize_content_view(true);
    }
    #[cfg(not(target_os = "macos"))]
    {
        viewport = viewport.with_decorations(false);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    if let Err(err) = eframe::run_native(
        "Lux Editor",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(App::new(cc.egui_ctx.clone(), font_loader)))
        }),
    ) {
        log::error!("failed to run Lux Editor: {err}");
    }
}
