//! Command group component: renders an optional group header plus its rows,
//! reporting row clicks and hover as messages.

use super::commands::PaletteTarget;
use super::rank::Group;
use super::row::{CommandRow, RowInput, RowMessage};
use crate::component::Component;
use eframe::egui;

pub(super) enum GroupMessage {
    Clicked(PaletteTarget),
    /// The pointer moved over the row at this flat index.
    Hovered(usize),
}

pub(super) struct GroupInput<'a> {
    pub group: &'a Group,
    /// Flat index of this group's first row, so hover maps to the panel's
    /// selection across all groups.
    pub base_index: usize,
    /// The panel's current flat selection index.
    pub selected: usize,
}

/// A group header plus its rows.
pub(super) struct CommandGroup;

impl Component for CommandGroup {
    type Message = GroupMessage;
    type Input<'a> = GroupInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<GroupMessage> {
        let mut messages = Vec::new();
        if let Some(title) = input.group.title {
            group_header(ui, title);
        }
        let mut row = CommandRow;
        for (offset, ranked) in input.group.commands.iter().enumerate() {
            let index = input.base_index + offset;
            let selected = index == input.selected;
            for message in row.render(
                ui,
                RowInput {
                    item: ranked,
                    selected,
                },
            ) {
                match message {
                    RowMessage::Clicked(target) => messages.push(GroupMessage::Clicked(target)),
                    RowMessage::Hovered => messages.push(GroupMessage::Hovered(index)),
                }
            }
        }
        messages
    }
}

/// A small uppercase group header (e.g. "Recently used").
fn group_header(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(title.to_uppercase())
            .small()
            .strong()
            .color(ui.visuals().weak_text_color()),
    );
    ui.add_space(2.0);
}
