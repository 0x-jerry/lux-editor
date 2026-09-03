use super::text_editor::{TextEditor, TextEditorState};
use super::welcome::WelcomeView;
use crate::config::Config;
use crate::events::{CustomEvent, DocumentEvent};
use crate::language::HighlightSnapshot;
use crate::ui::component::Component;
use crate::ui::highlight::snapshot_color;
use crate::ui::types::DocumentTab;
use eframe::egui;
use lux_core::Buffer;
use std::ops::Range;
use std::path::PathBuf;

/// The editor tab strip, welcome fallback and text area.
pub struct EditorView;

pub struct EditorViewState<'a> {
    pub workspace_path: Option<&'a PathBuf>,
    pub buffer: &'a Buffer,
    pub document_tabs: &'a [DocumentTab],
    pub active_document_index: usize,
    pub highlight_snapshot: &'a HighlightSnapshot,
    pub editor_config: &'a Config,
    /// All cursor positions as 1-based (line, column).
    pub carets: &'a [(usize, usize)],
    pub selection_ranges: &'a [Range<usize>],
    pub active_caret_index: usize,
    pub caret_visible: bool,
}

impl Component for EditorView {
    type Message = CustomEvent;
    type Input<'a> = EditorViewState<'a>;

    fn render(&mut self, ui: &mut egui::Ui, state: Self::Input<'_>) -> Vec<CustomEvent> {
        let EditorViewState {
            workspace_path,
            buffer,
            document_tabs,
            active_document_index,
            highlight_snapshot,
            editor_config,
            carets,
            selection_ranges,
            active_caret_index,
            caret_visible,
        } = state;
        let mut events = Vec::new();
        if workspace_path.is_none() && buffer.path().is_none() {
            let mut welcome_view = WelcomeView;
            events.extend(welcome_view.render(ui, editor_config));
            return events;
        }

        let editor_bg = snapshot_color(highlight_snapshot.background, ui.visuals().code_bg_color);

        egui::Frame::new()
            .fill(editor_bg)
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(6, 3))
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("document_tabs_scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for (index, tab) in document_tabs.iter().enumerate() {
                                let selected = index == active_document_index;
                                let (tab_response, close_clicked) =
                                    document_tab(ui, tab, selected, index);
                                if tab_response.clicked() {
                                    events.push(CustomEvent::Document(DocumentEvent::SwitchDocument(
                                        index,
                                    )));
                                }
                                if close_clicked {
                                    events.push(CustomEvent::Document(DocumentEvent::CloseDocument(
                                        index,
                                    )));
                                }
                            }
                        });
                    });
            });

        let mut text_editor = TextEditor;
        events.extend(
            text_editor
                .render(
                    ui,
                    TextEditorState {
                        buffer,
                        highlight_snapshot,
                        editor_config,
                        carets,
                        selection_ranges,
                        active_caret_index,
                        caret_visible,
                    },
                )
                .into_iter()
                .map(CustomEvent::Editing),
        );
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
) -> (egui::Response, bool) {
    let row_height = ui.spacing().interact_size.y;
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text_width =
        ui.fonts_mut(|fonts| fonts.glyph_width(&font, 'M')) * tab.title.chars().count() as f32;
    let close_size = 16.0;
    let tab_width = text_width + 12.0 + close_size + 8.0;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(tab_width, row_height), egui::Sense::click());

    let hovered = response.hovered();
    let painter = ui.painter();
    if selected || hovered {
        let bg = if selected {
            ui.visuals().widgets.inactive.bg_fill
        } else {
            ui.visuals().widgets.hovered.bg_fill
        };
        painter.rect_filled(rect, egui::CornerRadius::same(5), bg);
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

    let close_center = egui::pos2(rect.right() - close_size / 2.0 - 4.0, rect.center().y);
    let close_rect = egui::Rect::from_center_size(close_center, egui::vec2(close_size, close_size));
    let mut close_clicked = false;
    if hovered {
        let close_response = ui.interact(
            close_rect,
            ui.id().with(("tab_close", index)),
            egui::Sense::click(),
        );
        if close_response.hovered() {
            painter.rect_filled(
                close_rect,
                egui::CornerRadius::same((close_size / 2.0) as u8),
                ui.visuals().widgets.hovered.bg_fill,
            );
        }
        if close_response.clicked() {
            close_clicked = true;
        }
        painter.text(
            close_center,
            egui::Align2::CENTER_CENTER,
            "✕",
            egui::FontId::proportional(11.0),
            ui.visuals().weak_text_color(),
        );
    } else if tab.dirty {
        painter.circle_filled(
            egui::pos2(rect.right() - close_size / 2.0 - 5.0, rect.center().y),
            3.0,
            ui.visuals().hyperlink_color,
        );
    }

    (response, close_clicked)
}
