//! Scroll mirroring between two vertical panes that show the same document in
//! different shapes (the text editor and the markdown preview beside it). Their
//! content heights differ, so a shared pixel offset means nothing: the panes
//! share a scroll *fraction* instead. The pane under the pointer drives and
//! publishes the fraction; the other is pinned to it every frame.

use eframe::egui;

/// The pane a scroll measurement belongs to.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollPane {
    #[default]
    Editor,
    Preview,
}

/// A pane's viewport as last measured: where the pointer has to be for it to
/// drive, and how far it can scroll.
#[derive(Clone, Copy)]
struct PaneMeasure {
    rect: egui::Rect,
    /// `None` until the pane has rendered once; an unmeasured pane is never
    /// pinned to a guess.
    max_scroll: Option<f32>,
}

impl Default for PaneMeasure {
    fn default() -> Self {
        Self {
            rect: egui::Rect::NOTHING,
            max_scroll: None,
        }
    }
}

/// How far right of a pane's content a pointer still counts as being on its
/// scrollbar — dragging that scrollbar drives the pane too.
const SCROLLBAR_HIT: f32 = 12.0;

/// The shared scroll position of a pane pair. Owned by whoever lays both panes
/// out and handed to each of them around its render.
#[derive(Default)]
pub struct ScrollSync {
    driver: ScrollPane,
    fraction: f32,
    panes: [PaneMeasure; 2],
}

impl ScrollSync {
    /// Crown the pane the pointer is over. Skipped until both panes have been
    /// measured: the preview is laid out second and starts at the top, so
    /// letting a pointer that happens to rest over it crown it on that first
    /// frame would yank the editor up with it.
    pub fn update_driver(&mut self, ui: &egui::Ui) {
        if self.panes.iter().any(|pane| pane.max_scroll.is_none()) {
            return;
        }
        let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) else {
            return;
        };
        if self.pane(ScrollPane::Preview).rect.contains(pointer) {
            self.driver = ScrollPane::Preview;
        } else if self.pane(ScrollPane::Editor).rect.contains(pointer) {
            self.driver = ScrollPane::Editor;
        }
    }

    /// Make `pane` the driver because it scrolled for its own reasons (the
    /// editor revealing the caret) rather than mirroring the other pane.
    pub fn claim(&mut self, pane: ScrollPane) {
        self.driver = pane;
    }

    /// The offset to force on `pane` this frame; `None` when it drives.
    pub fn follow_offset(&self, pane: ScrollPane) -> Option<f32> {
        if pane == self.driver {
            return None;
        }
        self.pane(pane)
            .max_scroll
            .map(|max_scroll| self.fraction * max_scroll)
    }

    /// Record where `pane` ended up. Only the driver publishes the fraction;
    /// the follower's offset is a consequence of it.
    pub fn report(
        &mut self,
        pane: ScrollPane,
        offset: f32,
        inner_rect: egui::Rect,
        content_size: egui::Vec2,
    ) {
        let max_scroll = (content_size.y - inner_rect.height()).max(0.0);
        let measure = self.pane_mut(pane);
        measure.max_scroll = Some(max_scroll);
        measure.rect = egui::Rect::from_min_max(
            inner_rect.min,
            egui::pos2(inner_rect.right() + SCROLLBAR_HIT, inner_rect.bottom()),
        );
        if pane == self.driver && max_scroll > 0.0 {
            self.fraction = (offset / max_scroll).clamp(0.0, 1.0);
        }
    }

    fn pane(&self, pane: ScrollPane) -> &PaneMeasure {
        &self.panes[pane as usize]
    }

    fn pane_mut(&mut self, pane: ScrollPane) -> &mut PaneMeasure {
        &mut self.panes[pane as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0))
    }

    #[test]
    fn follower_mirrors_the_driver_fraction() {
        let mut sync = ScrollSync::default();
        // Nothing measured yet: no pane is pinned to a guess.
        assert_eq!(sync.follow_offset(ScrollPane::Editor), None);
        assert_eq!(sync.follow_offset(ScrollPane::Preview), None);

        // Editor: 2000px of content in a 100px viewport, a quarter scrolled.
        sync.report(
            ScrollPane::Editor,
            475.0,
            viewport(),
            egui::vec2(100.0, 2000.0),
        );
        // The driver scrolls free.
        assert_eq!(sync.follow_offset(ScrollPane::Editor), None);
        // The preview has not rendered yet, so it is not pinned either.
        assert_eq!(sync.follow_offset(ScrollPane::Preview), None);

        // Preview: 500px of content in the same viewport.
        sync.report(
            ScrollPane::Preview,
            0.0,
            viewport(),
            egui::vec2(100.0, 500.0),
        );
        // Mirrored to the same quarter of its own range: 0.25 * (500 - 100).
        assert_eq!(sync.follow_offset(ScrollPane::Preview), Some(100.0));

        // A caret reveal claims the editor and republishes the fraction.
        sync.claim(ScrollPane::Editor);
        sync.report(
            ScrollPane::Editor,
            950.0,
            viewport(),
            egui::vec2(100.0, 2000.0),
        );
        assert_eq!(sync.follow_offset(ScrollPane::Preview), Some(200.0));

        // The preview takes over: now the editor is the pinned pane.
        sync.claim(ScrollPane::Preview);
        sync.report(
            ScrollPane::Preview,
            400.0,
            viewport(),
            egui::vec2(100.0, 500.0),
        );
        assert_eq!(sync.follow_offset(ScrollPane::Preview), None);
        assert_eq!(sync.follow_offset(ScrollPane::Editor), Some(1900.0));
    }

    #[test]
    fn a_driver_that_stops_scrolling_keeps_the_fraction() {
        let mut sync = ScrollSync::default();
        sync.report(
            ScrollPane::Editor,
            475.0,
            viewport(),
            egui::vec2(100.0, 2000.0),
        );
        // The document shrank to fit the viewport: nothing left to scroll, so
        // the fraction stays put instead of dividing by zero.
        sync.report(ScrollPane::Editor, 0.0, viewport(), egui::vec2(100.0, 40.0));
        sync.report(
            ScrollPane::Preview,
            0.0,
            viewport(),
            egui::vec2(100.0, 800.0),
        );
        assert_eq!(sync.follow_offset(ScrollPane::Preview), Some(175.0));
    }
}
