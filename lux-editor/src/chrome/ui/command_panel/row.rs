//! Command row component: paints one two-line result row (selection, icon,
//! emphasized title, subtitle) and reports pointer interaction as messages.

use super::ROW_HEIGHT;
use super::commands::{CommandIcon, PaletteTarget};
use super::rank::RankedCommand;
use crate::chrome::ui::widgets::file_type_icon;
use crate::component::Component;
use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui_phosphor::regular::FOLDER;

/// Interaction a row reports back to the panel this frame.
pub(super) enum RowMessage {
    /// The row was clicked; run this target.
    Clicked(PaletteTarget),
    /// The pointer moved over this row; adopt it as the selection.
    Hovered,
}

pub(super) struct RowInput<'a> {
    pub item: &'a RankedCommand,
    pub selected: bool,
}

/// A single result row. Stateless; reports clicks and hover via [`RowMessage`].
pub(super) struct CommandRow;

impl Component for CommandRow {
    type Message = RowMessage;
    type Input<'a> = RowInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<RowMessage> {
        let mut messages = Vec::new();
        let row_height = ROW_HEIGHT;
        let width = ui.available_width();
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, row_height), egui::Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

        if input.selected {
            ui.painter()
                .rect_filled(rect, 0.0, ui.visuals().selection.bg_fill);
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.min, egui::vec2(2.0, row_height)),
                0.0,
                ui.visuals().hyperlink_color,
            );
            // Keep the selected row inside the visible area.
            let clip = ui.clip_rect();
            if rect.top() < clip.top() || rect.bottom() > clip.bottom() {
                ui.scroll_to_rect(rect, None);
            }
        } else if response.hovered() {
            ui.painter()
                .rect_filled(rect, 0.0, ui.visuals().widgets.hovered.bg_fill);
        }

        let weak = ui.visuals().weak_text_color();
        let accent = ui.visuals().hyperlink_color;
        let strong = ui.visuals().strong_text_color();
        let body_font = egui::TextStyle::Body.resolve(ui.style());

        // Text metrics first, so the icon can align with the title line and the
        // subtitle can sit below it with a clear gap.
        let text_left = rect.left() + 6.0 + 24.0 + 6.0;
        let text_right = rect.right() - 6.0;
        let title_color = if input.selected {
            strong
        } else {
            ui.visuals().text_color()
        };
        let job = title_layout_job(
            &input.item.command.title,
            input.item.match_spans.as_deref(),
            body_font.clone(),
            title_color,
            accent,
        );
        let galley = ui.painter().layout_job(job);
        let title_top = rect.top() + 5.0;
        let title_bottom = title_top + galley.size().y;

        // Icon column: vertically centered on the first (title) line.
        let icon_center = egui::pos2(rect.left() + 6.0 + 12.0, title_top + galley.size().y * 0.5);
        match &input.item.command.icon {
            CommandIcon::Phosphor(glyph) => {
                let color = if input.selected { accent } else { weak };
                ui.painter().text(
                    icon_center,
                    egui::Align2::CENTER_CENTER,
                    *glyph,
                    body_font.clone(),
                    color,
                );
            }
            CommandIcon::Folder => {
                let color = if input.selected { accent } else { weak };
                ui.painter().text(
                    icon_center,
                    egui::Align2::CENTER_CENTER,
                    FOLDER,
                    body_font.clone(),
                    color,
                );
            }
            CommandIcon::File(path) => {
                let (glyph, color) = file_type_icon(ui.visuals().dark_mode, path);
                ui.painter().text(
                    icon_center,
                    egui::Align2::CENTER_CENTER,
                    glyph.to_string(),
                    egui::FontId::new(body_font.size, egui::FontFamily::Name("devicons".into())),
                    color,
                );
            }
        }

        ui.painter().galley(
            egui::pos2(text_left, title_top),
            galley.clone(),
            title_color,
        );

        if let Some(subtitle) = &input.item.command.subtitle {
            let subtitle_font = egui::TextStyle::Small.resolve(ui.style());
            let sub_top = title_bottom + 4.0;
            let sub_clip = egui::Rect::from_min_max(
                egui::pos2(text_left, rect.top()),
                egui::pos2(text_right, rect.bottom()),
            );
            ui.painter_at(sub_clip).text(
                egui::pos2(text_left, sub_top),
                egui::Align2::LEFT_CENTER,
                subtitle,
                subtitle_font,
                weak,
            );
        }

        if response.clicked() {
            messages.push(RowMessage::Clicked(input.item.command.target.clone()));
        }
        // Keyboard navigation wins over a parked pointer, so only adopt a row
        // via hover when the pointer actually moved this frame. `motion()` is
        // sticky in egui (Some(ZERO) forever after the first move), so a
        // non-zero delta is the only reliable "moved this frame" signal.
        if response.hovered()
            && ui.input(|i| {
                i.pointer
                    .motion()
                    .is_some_and(|motion| motion != egui::Vec2::ZERO)
            })
        {
            messages.push(RowMessage::Hovered);
        }

        messages
    }
}

/// Layout the title line, highlighting matched characters in `accent`.
fn title_layout_job(
    title: &str,
    spans: Option<&[(usize, usize)]>,
    font: egui::FontId,
    base_color: egui::Color32,
    accent: egui::Color32,
) -> LayoutJob {
    let mut job = LayoutJob {
        halign: egui::Align::LEFT,
        ..Default::default()
    };
    let chars: Vec<char> = title.chars().collect();
    let is_matched = |i: usize| {
        spans.is_some_and(|spans| spans.iter().any(|&(start, end)| i >= start && i < end))
    };

    let base = TextFormat::simple(font.clone(), base_color);
    let highlighted = TextFormat::simple(font, accent);

    let mut run_start = 0usize;
    let mut run_matched = is_matched(0);
    for i in 1..chars.len() {
        let matched = is_matched(i);
        if matched != run_matched {
            let segment: String = chars[run_start..i].iter().collect();
            job.append(
                &segment,
                0.0,
                if run_matched {
                    highlighted.clone()
                } else {
                    base.clone()
                },
            );
            run_start = i;
            run_matched = matched;
        }
    }
    let segment: String = chars[run_start..].iter().collect();
    job.append(&segment, 0.0, if run_matched { highlighted } else { base });
    job
}
