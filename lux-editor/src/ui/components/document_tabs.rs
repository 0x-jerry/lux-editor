use crate::events::DocumentEvent;
use crate::ui::component::Component;
use crate::ui::types::DocumentTab;
use eframe::egui;
use egui_phosphor::regular::X;

/// The editor document tab strip.
pub struct DocumentTabsView;

pub struct DocumentTabsInput<'a> {
    pub tabs: &'a [DocumentTab],
    pub active_index: usize,
    pub background: egui::Color32,
}

impl Component for DocumentTabsView {
    type Message = DocumentEvent;
    type Input<'a> = DocumentTabsInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<DocumentEvent> {
        let mut events = Vec::new();
        egui::Frame::new()
            .fill(input.background)
            .inner_margin(egui::Margin::same(0))
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("document_tabs_scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for (index, tab) in input.tabs.iter().enumerate() {
                                let (clicked, close_clicked) =
                                    document_tab(ui, tab, index == input.active_index, index);
                                if clicked {
                                    events.push(DocumentEvent::SwitchDocument(index));
                                }
                                if close_clicked {
                                    events.push(DocumentEvent::CloseDocument(index));
                                }
                            }
                        });
                    });
            });
        events
    }
}

/// A document tab: filled when active with an accent top bar; the close button
/// appears on hover, and an accent dot marks unsaved changes otherwise.
fn document_tab(
    ui: &mut egui::Ui,
    tab: &DocumentTab,
    selected: bool,
    index: usize,
) -> (bool, bool) {
    let row_height = ui.spacing().interact_size.y;
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text_width =
        ui.fonts_mut(|fonts| fonts.glyph_width(&font, 'M')) * tab.title.chars().count() as f32;
    let close_size = 16.0;
    let tab_width = text_width + 12.0 + close_size + 8.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(tab_width, row_height), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    let close_center = egui::pos2(rect.right() - close_size / 2.0 - 4.0, rect.center().y);
    let close_rect = egui::Rect::from_center_size(close_center, egui::vec2(close_size, close_size));
    // Allocated after the tab so the button stays on top of it, but that also
    // means the tab loses hover whenever the pointer is over the button —
    // combine both hover states so the button never flickers or disappears,
    // and only one of the two receives the click.
    let close_response = ui
        .interact(
            close_rect,
            ui.id().with(("tab_close", index)),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    let hovered = response.hovered() || close_response.hovered();
    let painter = ui.painter();
    if selected || hovered {
        let bg = if selected {
            ui.visuals().widgets.inactive.bg_fill
        } else {
            ui.visuals().widgets.hovered.bg_fill
        };
        painter.rect_filled(rect, 0.0, bg);
    }
    if selected {
        painter.rect_filled(
            egui::Rect::from_min_max(rect.left_top(), egui::pos2(rect.right(), rect.top() + 2.0)),
            0.0,
            ui.visuals().hyperlink_color,
        );
    }

    let text_color = if selected {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    painter.text(
        egui::pos2(rect.left() + 8.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &tab.title,
        font,
        text_color,
    );

    if hovered {
        painter.text(
            close_center,
            egui::Align2::CENTER_CENTER,
            X,
            egui::FontId::proportional(11.0),
            crate::ui::widgets::icon_text_color(ui, close_response.hovered()),
        );
    } else if tab.dirty {
        painter.circle_filled(
            egui::pos2(rect.right() - close_size / 2.0 - 5.0, rect.center().y),
            3.0,
            ui.visuals().hyperlink_color,
        );
    }

    (response.clicked(), close_response.clicked())
}
