use crate::chrome::ui::{FileBinaryInput, FileBinaryView};
use crate::component::Component;
use eframe::egui;
use std::path::Path;

const IMAGE_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico"];

pub fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            IMAGE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
        })
}

/// The `file://` URI of a path. Rendering and cache invalidation must build
/// the exact same string or `forget_image` misses the cached entry.
pub(crate) fn file_uri(path: &Path) -> String {
    format!("file://{}", path.display())
}

/// What the image view computed this frame, published for the status bar:
/// native pixel size and the displayed/native scale. Keyed per URI in egui's
/// data (`ImageStatus::data_id`), written each render, read a frame later.
#[derive(Clone, Copy)]
pub struct ImageStatus {
    pub pixels: egui::Vec2,
    pub scale: f32,
}

impl ImageStatus {
    pub fn data_id(uri: &str) -> egui::Id {
        egui::Id::new(uri).with("image_status")
    }
}

/// Editor-area page for a tab whose binary file is an image: the picture
/// replaces the binary guide page. The tab stays a binary tab, so editing and
/// saving remain refused. A file that fails to decode falls back to the guide
/// page — a corrupt `.png` is just another unshowable binary.
pub struct FileImageView;

pub struct FileImageInput<'a> {
    pub path: &'a Path,
}

/// Zoom is a multiplier on the fit-to-window scale; offset pans the centered
/// image. Keyed per URI in egui's data, since the view itself is built fresh
/// every frame.
#[derive(Clone, Copy)]
struct ZoomState {
    zoom: f32,
    offset: egui::Vec2,
}

impl Default for ZoomState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: egui::Vec2::ZERO,
        }
    }
}

const ZOOM_MIN: f32 = 0.05;
const ZOOM_MAX: f32 = 40.0;

impl Component for FileImageView {
    type Message = ();
    type Input<'a> = FileImageInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<()> {
        let FileImageInput { path } = input;
        let uri = file_uri(path);
        let image = egui::Image::new(egui::ImageSource::Uri(std::borrow::Cow::Owned(uri.clone())));

        // The area below the tab strip's cursor: painting and clipping stay
        // inside it, so a panned image never covers the tabs.
        let image_area = ui.available_rect_before_wrap();

        match image.load_for_size(ui.ctx(), image_area.size()) {
            Ok(egui::load::TexturePoll::Pending { .. }) => {
                ui.put(
                    egui::Rect::from_center_size(image_area.center(), egui::Vec2::splat(28.0)),
                    egui::Spinner::new().size(28.0),
                );
            }
            Err(_) => {
                let mut binary_view = FileBinaryView;
                binary_view.render(ui, FileBinaryInput { path });
            }
            Ok(egui::load::TexturePoll::Ready { texture }) => {
                let response = ui.allocate_rect(image_area, egui::Sense::click_and_drag());
                self.render_image(ui, &image, &uri, &response, image_area, texture.size);
            }
        }

        Vec::new()
    }
}

impl FileImageView {
    fn render_image(
        &self,
        ui: &mut egui::Ui,
        image: &egui::Image<'_>,
        uri: &str,
        response: &egui::Response,
        image_area: egui::Rect,
        image_size: egui::Vec2,
    ) {
        // Small images rest at native size, large ones shrink to the window.
        let fit = (image_area.width() / image_size.x)
            .min(image_area.height() / image_size.y)
            .min(1.0);

        let id = egui::Id::new(uri);
        let mut state: ZoomState = ui.data(|data| data.get_temp(id)).unwrap_or_default();

        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let zoomed = (state.zoom * (scroll * 0.01).exp()).clamp(ZOOM_MIN, ZOOM_MAX);
                // Keep the image point under the pointer pinned while zooming.
                if let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) {
                    let center = image_area.center();
                    let point = (pointer - center - state.offset) / (fit * state.zoom);
                    state.offset = pointer - center - point * (fit * zoomed);
                }
                state.zoom = zoomed;
                ui.ctx().request_repaint();
            }
        }
        if response.dragged() {
            state.offset += response.drag_delta();
            ui.ctx().request_repaint();
        }
        if response.double_clicked() {
            state = ZoomState::default();
        }
        ui.data_mut(|data| data.insert_temp(id, state));

        let rect = egui::Rect::from_center_size(
            image_area.center() + state.offset,
            image_size * (fit * state.zoom),
        );
        ui.data_mut(|data| {
            data.insert_temp(
                ImageStatus::data_id(uri),
                ImageStatus {
                    pixels: image_size,
                    scale: fit * state.zoom,
                },
            )
        });
        ui.scope(|ui| {
            ui.set_clip_rect(image_area);
            image.paint_at(ui, rect);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_extensions_are_recognized_case_insensitively() {
        assert!(is_image_path(Path::new("/ws/photo.PNG")));
        assert!(is_image_path(Path::new("/ws/photo.jpg")));
        assert!(is_image_path(Path::new("/ws/photo.Jpeg")));
        assert!(is_image_path(Path::new("/ws/anim.gif")));
        assert!(is_image_path(Path::new("/ws/pic.webp")));
        assert!(is_image_path(Path::new("/ws/pic.bmp")));
        assert!(is_image_path(Path::new("/ws/pic.ico")));
        assert!(!is_image_path(Path::new("/ws/code.rs")));
        assert!(!is_image_path(Path::new("/ws/notes")));
        assert!(!is_image_path(Path::new("/ws/archive.zip")));
    }
}
