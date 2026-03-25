mod app;
mod config;
mod events;
mod file_tree;
mod file_watcher;
mod language;
mod state;
mod ui;

use app::App;
use events::CustomEvent;
use winit::event_loop::EventLoop;

pub fn main() {
    env_logger::init();

    let event_loop = EventLoop::<CustomEvent>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);
    event_loop.run_app(&mut app).unwrap();
}
