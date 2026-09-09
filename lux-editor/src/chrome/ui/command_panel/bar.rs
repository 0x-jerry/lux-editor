//! Command panel top search bar and bottom hint bar.

use crate::component::Component;
use eframe::egui;
use egui_phosphor::regular::{
    ARROWS_DOWN_UP, ARROW_ELBOW_DOWN_RIGHT, ARROW_LEFT, MAGNIFYING_GLASS, X,
};

/// Top bar: the search field that filters the command list in place.
pub(super) struct CommandHeader;

pub(super) struct HeaderInput<'a> {
    pub query: &'a mut String,
    pub hint: &'static str,
}

impl Component for CommandHeader {
    type Message = ();
    type Input<'a> = HeaderInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<()> {
        let weak = ui.visuals().weak_text_color();
        ui.horizontal(|ui| {
            ui.add_space(2.0);
            ui.label(egui::RichText::new(MAGNIFYING_GLASS).color(weak));
            let field = ui.add(
                egui::TextEdit::singleline(input.query)
                    .id_salt("command_panel_search")
                    .hint_text(input.hint)
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
            field.request_focus();
        });
        Vec::new()
    }
}

/// Which hint icons the footer shows.
pub(super) enum FooterHint {
    Root,
    RecentList,
}

/// Bottom bar: key-hint icons + labels on the left, result count on the right.
pub(super) struct CommandFooter;

pub(super) struct FooterInput {
    pub hint: FooterHint,
    pub result_count: usize,
}

impl Component for CommandFooter {
    type Message = ();
    type Input<'a> = FooterInput;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<()> {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            hint(ui, ARROWS_DOWN_UP, "Navigate");
            match input.hint {
                FooterHint::Root => hint(ui, ARROW_ELBOW_DOWN_RIGHT, "Run"),
                FooterHint::RecentList => hint(ui, ARROW_ELBOW_DOWN_RIGHT, "Open"),
            }
            match input.hint {
                FooterHint::Root => hint(ui, X, "Close"),
                FooterHint::RecentList => hint(ui, ARROW_LEFT, "Back"),
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let weak = ui.visuals().weak_text_color();
                ui.label(
                    egui::RichText::new(format!("{} results", input.result_count))
                        .small()
                        .color(weak),
                );
            });
        });
        Vec::new()
    }
}

/// One hint group: an icon and its label combined into a single composed label.
fn hint(ui: &mut egui::Ui, glyph: &str, text: &str) {
    let weak = ui.visuals().weak_text_color();
    let mut job = egui::text::LayoutJob::default();
    for piece in [glyph, &format!("  {text}")] {
        egui::RichText::new(piece)
            .small()
            .color(weak)
            .append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::LEFT);
    }
    ui.label(job);
}
