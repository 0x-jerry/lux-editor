use super::state::{TabMeta, TabState};
use crate::component::Component;
use crate::events::DocumentEvent;
use eframe::egui;
use egui_phosphor::regular::X;

/// The shared tab strip input: content-agnostic metadata plus which tab is
/// active. The strip never looks inside a tab's content.
pub struct TabStripInput<'a> {
    pub tabs: &'a [TabMeta],
    pub active_id: u64,
    pub background: egui::Color32,
}

/// The shared tab strip: one fixed-width tab per entry.
pub(crate) struct TabStripView;

impl Component for TabStripView {
    type Message = DocumentEvent;
    type Input<'a> = TabStripInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<DocumentEvent> {
        let mut events = Vec::new();
        // Reveal the active tab only when the active tab changes, so switching
        // scrolls it into view without re-yanking it back if the user scrolls
        // away. Keyed by the active tab's id (not its index — closing earlier
        // tabs shifts indices but is not a switch).
        let active_state_id = ui.id().with("tab_strip_active_state");
        let previous_id = ui.data(|data| data.get_temp::<u64>(active_state_id));
        let reveal_active = previous_id != Some(input.active_id);
        ui.data_mut(|data| data.insert_temp(active_state_id, input.active_id));
        let strip = egui::Frame::new()
            .fill(input.background)
            .inner_margin(egui::Margin::same(0))
            .show(ui, |ui| {
                // Wheel over the strip scrolls it horizontally without Shift;
                // egui reads only the wheel axis matching a single-direction
                // scroll area unless this style flag is set.
                ui.style_mut().always_scroll_the_only_direction = true;
                egui::ScrollArea::horizontal()
                    .id_salt("tab_strip_scroll")
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            let mut tab_view = TabView;
                            for index in 0..input.tabs.len() {
                                let tab = &input.tabs[index];
                                // The pristine scratch tab is the welcome page's
                                // placeholder, not a document; hide it while a
                                // real tab holds focus.
                                if tab.provisional && tab.id != input.active_id {
                                    continue;
                                }
                                events.extend(tab_view.render(
                                    ui,
                                    TabInput {
                                        tab,
                                        index,
                                        selected: tab.id == input.active_id,
                                        reveal_active: tab.id == input.active_id && reveal_active,
                                    },
                                ));
                            }
                        });
                    });
            });
        // Hairline under the strip, separating it from the content below.
        ui.painter().hline(
            strip.response.rect.x_range(),
            strip.response.rect.bottom(),
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        );
        events
    }
}

/// A single tab: filled when active with an accent top bar; the close button
/// appears on hover, an accent dot marks unsaved changes otherwise, and a
/// `Warning` state (e.g. a deleted file) strikes the title through.
///
/// Every tab is fixed-width; long titles elide to it.
const TAB_WIDTH: f32 = 120.0;

/// Minimum tab row height. Tabs stay at least this tall for comfortable hit
/// targets, and grow with the global interact size (theme) if that is larger.
const TAB_HEIGHT_MIN: f32 = 32.0;

struct TabView;

struct TabInput<'a> {
    tab: &'a TabMeta,
    /// Position in the strip; identifies this tab's close-handle and is echoed
    /// back in the emitted event.
    index: usize,
    selected: bool,
    /// True only for the active tab on the frame it just became active.
    reveal_active: bool,
}

impl Component for TabView {
    type Message = DocumentEvent;
    type Input<'a> = TabInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<DocumentEvent> {
        let TabInput {
            tab,
            index,
            selected,
            reveal_active,
        } = input;
        let mut events = Vec::new();
        let row_height = ui.spacing().interact_size.y.max(TAB_HEIGHT_MIN);
        let font = egui::TextStyle::Button.resolve(ui.style());
        let warning = tab.state == TabState::Warning;
        let text_color = if warning {
            ui.visuals().warn_fg_color
        } else if selected {
            ui.visuals().strong_text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        // Strikethrough needs a laid-out galley; elision to the fixed width
        // does too, so the job is built with a truncation wrap.
        let close_size = 16.0;
        let tab_width = TAB_WIDTH;
        let title_max_width = tab_width - 8.0 - close_size - 8.0;
        let title = {
            let mut job = egui::text::LayoutJob {
                wrap: egui::text::TextWrapping::truncate_at_width(title_max_width),
                ..Default::default()
            };
            job.append(
                &tab.title,
                0.0,
                egui::text::TextFormat {
                    font_id: font.clone(),
                    color: text_color,
                    strikethrough: if warning {
                        egui::Stroke::new(1.0, text_color)
                    } else {
                        egui::Stroke::NONE
                    },
                    ..Default::default()
                },
            );
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(tab_width, row_height), egui::Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

        // Reveal the active tab on switch only when it is scrolled out of view.
        if selected && reveal_active {
            response.scroll_to_me(None);
        }

        let close_center = egui::pos2(rect.right() - close_size / 2.0 - 4.0, rect.center().y);
        let close_rect =
            egui::Rect::from_center_size(close_center, egui::vec2(close_size, close_size));
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
                egui::Rect::from_min_max(
                    rect.left_top(),
                    egui::pos2(rect.right(), rect.top() + 2.0),
                ),
                0.0,
                ui.visuals().hyperlink_color,
            );
        }

        painter.galley(
            egui::pos2(rect.left() + 8.0, rect.center().y - title.size().y / 2.0),
            title,
            text_color,
        );

        if hovered {
            if tab.closable {
                painter.text(
                    close_center,
                    egui::Align2::CENTER_CENTER,
                    X,
                    egui::FontId::proportional(11.0),
                    crate::chrome::ui::widgets::icon_text_color(ui, close_response.hovered()),
                );
            }
        } else if tab.dirty {
            painter.circle_filled(
                egui::pos2(rect.right() - close_size / 2.0 - 5.0, rect.center().y),
                3.0,
                ui.visuals().hyperlink_color,
            );
        }

        if response.clicked() {
            events.push(DocumentEvent::SwitchTab(tab.id));
        }
        if tab.closable && close_response.clicked() {
            events.push(DocumentEvent::CloseTab(tab.id));
        }
        events
    }
}
