//! Command palette overlay (Quick-Open style). A floating, rounded card pinned
//! just below the title bar with a search field on top and a filtered result
//! list beneath. Commands are registered in [`commands`]; the panel ranks
//! ([`rank`]) and paints ([`row`]) them, then runs whatever is selected.

mod bar;
mod commands;
mod group;
mod rank;
mod row;

use crate::component::Component;
use crate::events::{AppEvent, CustomEvent};
use crate::settings::Config;
use bar::{CommandFooter, CommandHeader, FooterHint, FooterInput, HeaderInput};
use commands::{Command, CommandKind, PaletteItem, PaletteTarget};
pub(crate) use commands::PaletteContext;
use eframe::egui;
use group::{CommandGroup, GroupInput, GroupMessage};
use rank::{rank_commands, Group, RankedCommand};

/// Vertical offset of the card's top edge below the window top. The title bar
/// is 32px tall; 40px leaves an 8px gap.
const PANEL_TOP_OFFSET: f32 = 40.0;
/// Height of one two-line result row.
const ROW_HEIGHT: f32 = 44.0;
/// Fixed width of the card.
const PANEL_WIDTH: f32 = 640.0;

/// Command palette.
#[derive(Default)]
pub struct CommandPanel {
    state: CommandPanelState,
}

/// Everything the palette needs this frame.
pub struct CommandPanelInput<'a> {
    pub config: &'a Config,
    /// Frame context that gates command visibility.
    pub context: PaletteContext,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum CommandPanelMode {
    #[default]
    Root,
    RecentList,
}

#[derive(Default)]
struct CommandPanelState {
    open: bool,
    query: String,
    selected: usize,
    mode: CommandPanelMode,
    recent_used: Vec<&'static str>,
}

impl CommandPanel {
    pub fn open(&self) -> bool {
        self.state.open
    }

    pub fn toggle(&mut self) {
        if self.state.open {
            self.close();
            return;
        }
        self.state.open = true;
        self.state.query.clear();
        self.state.selected = 0;
        self.state.mode = CommandPanelMode::Root;
    }

    fn close(&mut self) {
        self.state.open = false;
        self.state.query.clear();
        self.state.selected = 0;
        self.state.mode = CommandPanelMode::Root;
    }

    fn open_root_commands(&mut self) {
        self.state.mode = CommandPanelMode::Root;
        self.state.query.clear();
        self.state.selected = 0;
    }

    fn display_commands(&self, input: &CommandPanelInput<'_>) -> Vec<Group> {
        if self.state.mode == CommandPanelMode::RecentList {
            let ranked = rank_commands(&self.state.query, commands::recent_items(input.config));
            return vec![Group {
                title: Some("Recent items"),
                commands: ranked,
            }];
        }

        let ranked = rank_commands(&self.state.query, self.root_commands(&input.context));
        if self.show_recent_used_section() {
            let recent = commands::items_from_ids(&self.state.recent_used, &input.context)
                .into_iter()
                .map(|command| RankedCommand {
                    command,
                    score: i32::MAX,
                    match_spans: None,
                })
                .collect();
            let mut all = Vec::with_capacity(ranked.len());
            for ranked in ranked {
                if let PaletteTarget::Registered(command) = &ranked.command.target
                    && self.state.recent_used.iter().any(|id| id == &command.id)
                {
                    continue;
                }
                all.push(ranked);
            }
            vec![
                Group {
                    title: Some("Recently used"),
                    commands: recent,
                },
                Group {
                    title: Some("All commands"),
                    commands: all,
                },
            ]
        } else {
            vec![Group {
                title: None,
                commands: ranked,
            }]
        }
    }

    fn show_recent_used_section(&self) -> bool {
        self.state.mode == CommandPanelMode::Root
            && self.state.query.trim().is_empty()
            && !self.state.recent_used.is_empty()
    }

    fn root_commands(&self, context: &PaletteContext) -> Vec<PaletteItem> {
        commands::COMMANDS
            .iter()
            .filter(|command| (command.available)(context))
            .map(Command::palette_item)
            .collect()
    }

    fn query_hint(&self) -> &'static str {
        if self.state.mode == CommandPanelMode::RecentList {
            "Select recent item"
        } else {
            "Type a command"
        }
    }

    fn footer_hint(&self) -> FooterHint {
        if self.state.mode == CommandPanelMode::RecentList {
            FooterHint::RecentList
        } else {
            FooterHint::Root
        }
    }

    /// Record a run command in the capped "Recently used" list. Dynamic recent
    /// items are never shown there, so only registered commands are stored.
    fn remember(&mut self, target: &PaletteTarget) {
        let PaletteTarget::Registered(command) = target else {
            return;
        };
        self.state.recent_used.retain(|id| *id != command.id);
        self.state.recent_used.insert(0, command.id);
        if self.state.recent_used.len() > 5 {
            self.state.recent_used.truncate(5);
        }
    }
}

impl Component for CommandPanel {
    type Message = CustomEvent;
    type Input<'a> = CommandPanelInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<CustomEvent> {
        if !self.state.open {
            return Vec::new();
        }

        let mut events = Vec::new();
        let mut should_close = false;
        let mut pending_target: Option<PaletteTarget> = None;
        let query_hint = self.query_hint();
        let footer_hint = self.footer_hint();
        let groups = self.display_commands(&input);

        // Flat concatenation drives keyboard navigation and the empty check.
        let flat: Vec<&RankedCommand> = groups
            .iter()
            .flat_map(|group| group.commands.iter())
            .collect();
        let result_count = flat.len();
        if result_count == 0 {
            self.state.selected = 0;
        } else if self.state.selected >= result_count {
            self.state.selected = result_count - 1;
        }

        let ctx = ui.ctx().clone();

        let card = egui::Area::new(egui::Id::new("command_panel"))
            .order(egui::Order::Foreground)
            .movable(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, PANEL_TOP_OFFSET))
            .show(&ctx, |ui| {
                let width = PANEL_WIDTH;
                egui::Frame::NONE
                    .corner_radius(egui::CornerRadius::same(8))
                    .fill(ui.visuals().window_fill)
                    .stroke(egui::Stroke::new(
                        1.0,
                        ui.visuals().widgets.noninteractive.bg_stroke.color,
                    ))
                    .shadow(egui::Shadow {
                        offset: [0, 6],
                        blur: 32,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(70),
                    })
                    .inner_margin(egui::Margin::symmetric(10, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(width);
                        let weak = ui.visuals().weak_text_color();

                        // Search row: icon + field.
                        let mut header = CommandHeader;
                        header.render(
                            ui,
                            HeaderInput {
                                query: &mut self.state.query,
                                hint: query_hint,
                            },
                        );
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Results.
                        if result_count == 0 {
                            ui.add_space(12.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("No matching commands")
                                        .italics()
                                        .color(weak),
                                );
                            });
                            ui.add_space(12.0);
                        } else {
                            let max_list = (10.0 * ROW_HEIGHT)
                                .min(ctx.content_rect().height() * 0.6);
                            egui::ScrollArea::vertical()
                                .max_height(max_list)
                                .show(ui, |ui| {
                                    let mut group_view = CommandGroup;
                                    let mut global = 0usize;
                                    for group in &groups {
                                        let count = group.commands.len();
                                        for message in group_view.render(
                                            ui,
                                            GroupInput {
                                                group,
                                                base_index: global,
                                                selected: self.state.selected,
                                            },
                                        ) {
                                            match message {
                                                GroupMessage::Clicked(target) => {
                                                    pending_target = Some(target);
                                                }
                                                GroupMessage::Hovered(index) => {
                                                    self.state.selected = index;
                                                }
                                            }
                                        }
                                        global += count;
                                    }
                                });
                        }

                        // Footer hint bar (with result count on the right).
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        let mut footer = CommandFooter;
                        footer.render(
                            ui,
                            FooterInput {
                                hint: footer_hint,
                                result_count,
                            },
                        );
                    });
            });
        let card_rect = card.response.rect;

        // Keyboard navigation.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.state.mode == CommandPanelMode::RecentList {
                self.open_root_commands();
            } else {
                should_close = true;
            }
        }
        if result_count > 0 {
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                self.state.selected = (self.state.selected + 1) % result_count;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                self.state.selected = if self.state.selected == 0 {
                    result_count - 1
                } else {
                    self.state.selected - 1
                };
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                pending_target = Some(flat[self.state.selected].command.target.clone());
            }
        }

        // Clicking anywhere outside the card closes it (Quick-Open behavior).
        if ctx.input(|i| i.pointer.primary_clicked())
            && ctx
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|pos| !card_rect.contains(pos))
        {
            should_close = true;
        }

        if should_close {
            self.close();
            return events;
        }

        if let Some(target) = pending_target {
            match &target {
                PaletteTarget::Registered(command) if command.kind == CommandKind::ShowRecents => {
                    self.remember(&target);
                    self.state.mode = CommandPanelMode::RecentList;
                    self.state.query.clear();
                    self.state.selected = 0;
                }
                PaletteTarget::Registered(command) => {
                    self.remember(&target);
                    (command.run)(&mut events);
                    self.close();
                }
                PaletteTarget::OpenRecent { path, is_dir } => {
                    if *is_dir {
                        if path.is_dir() {
                            events.push(CustomEvent::App(AppEvent::OpenFolder(path.clone())));
                        }
                    } else if path.is_file() {
                        events.push(CustomEvent::App(AppEvent::OpenFile(path.clone())));
                    }
                    self.close();
                }
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::{commands, CommandKind, CommandPanel, PaletteTarget};
    use std::path::PathBuf;

    #[test]
    fn recent_used_keeps_latest_first_and_caps() {
        let mut panel = CommandPanel::default();
        let targets = ["save-file", "open-folder", "toggle-sidebar", "format-file"]
            .iter()
            .map(|id| PaletteTarget::Registered(commands::by_id(id).unwrap()))
            .collect::<Vec<_>>();
        for target in &targets {
            panel.remember(target);
        }
        assert_eq!(
            panel.state.recent_used,
            vec!["format-file", "toggle-sidebar", "open-folder", "save-file"]
        );
        // Re-running an entry moves it to the front without duplicating.
        panel.remember(&targets[0]);
        assert_eq!(panel.state.recent_used.len(), 4);
        assert_eq!(panel.state.recent_used[0], "save-file");
    }

    #[test]
    fn recent_used_skips_recent_item_targets() {
        let mut panel = CommandPanel::default();
        // Opening a recent item must not pollute the capped "Recently used" list.
        panel.remember(&PaletteTarget::OpenRecent {
            path: PathBuf::from("/tmp/project"),
            is_dir: true,
        });
        assert!(panel.state.recent_used.is_empty());
    }

    #[test]
    fn open_recently_command_has_show_recents_kind() {
        let command = commands::by_id("open-recently").unwrap();
        assert_eq!(command.kind, CommandKind::ShowRecents);
    }
}
