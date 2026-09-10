//! Small, reusable, app-agnostic widgets. They emit their own domain message
//! enums; the shell root maps them into the app's event pipeline.

pub(crate) use file_icon::file_type_icon;
pub use icon_button::{icon_button, icon_text_color};
pub(crate) use prompt::prompt_frame;
pub use status_bar::{StatusBar, StatusBarData};
pub use title_bar::{TitleBar, TitleBarData, window_resize_handle};

mod file_icon;
mod icon_button;
mod prompt;
mod status_bar;
mod title_bar;
