//! Small, reusable, app-agnostic widgets. They emit their own domain message
//! enums; the shell root maps them into the app's event pipeline.

pub use icon_button::{icon_button, icon_text_color};
pub use status_bar::{StatusBar, StatusBarData, StatusBarSection};
pub use title_bar::{TitleBar, TitleBarData, window_resize_handle};

mod icon_button;
mod status_bar;
mod title_bar;
