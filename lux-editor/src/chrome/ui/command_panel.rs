//! Command palette overlay (Quick-Open style). A floating, rounded card pinned
//! just below the title bar with a search field on top and a filtered result
//! list beneath. The only stateful component in the shell; the app owns the
//! instance so its query/selection survive between frames.

use crate::chrome::ui::widgets::file_type_icon;
use crate::component::Component;
use crate::events::{AppEvent, CustomEvent, DocumentEvent, ShellEvent};
use crate::settings::Config;
use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui_phosphor::regular::{
    CLOCK_COUNTER_CLOCKWISE, FILES, FLOPPY_DISK, FOLDER, FOLDER_SIMPLE, GEAR_SIX,
    MAGNIFYING_GLASS, SIDEBAR_SIMPLE, TEXT_ALIGN_LEFT, TRASH,
};
use std::path::PathBuf;

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
    recent_used_actions: Vec<CommandPanelAction>,
}

#[derive(Clone, Debug)]
enum CommandPanelAction {
    SaveFile,
    FormatFile,
    OpenFile,
    OpenRecently,
    OpenFolder,
    SwitchToConfiguration,
    ToggleSidebar,
    ClearRecentItems,
    OpenRecentItem { path: PathBuf, is_dir: bool },
}

/// Which glyph a row leads with: a phosphor glyph, a folder, or a themed
/// devicon for a specific file path.
#[derive(Clone)]
enum CommandIcon {
    Phosphor(&'static str),
    Folder,
    File(PathBuf),
}

#[derive(Clone)]
struct CommandPanelCommand {
    title: String,
    /// Secondary line under the title (category for commands, path for recents).
    subtitle: Option<String>,
    keywords: Vec<String>,
    icon: CommandIcon,
    action: CommandPanelAction,
}

#[derive(Clone)]
struct RankedCommand {
    command: CommandPanelCommand,
    score: i32,
    /// Char-index ranges within `title` that matched the query (emphasis).
    match_spans: Option<Vec<(usize, usize)>>,
}

/// A run of commands rendered under an optional group header.
struct CommandGroup {
    title: Option<&'static str>,
    commands: Vec<RankedCommand>,
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

    fn display_commands(&self, config: &Config) -> Vec<CommandGroup> {
        if self.state.mode == CommandPanelMode::RecentList {
            let ranked = rank_commands(&self.state.query, self.current_commands(config));
            return vec![CommandGroup {
                title: Some("Recent items"),
                commands: ranked,
            }];
        }

        let ranked = rank_commands(&self.state.query, self.current_commands(config));
        if self.show_recent_used_section() {
            let recent = build_recent_used_commands(&self.state.recent_used_actions);
            let recent_keys = recent
                .iter()
                .map(|ranked| action_key(&ranked.command.action))
                .collect::<Vec<_>>();
            let mut all = Vec::with_capacity(ranked.len());
            for ranked in ranked {
                let key = action_key(&ranked.command.action);
                if !recent_keys.iter().any(|recent_key| recent_key == &key) {
                    all.push(ranked);
                }
            }
            vec![
                CommandGroup {
                    title: Some("Recently used"),
                    commands: recent,
                },
                CommandGroup {
                    title: Some("All commands"),
                    commands: all,
                },
            ]
        } else {
            vec![CommandGroup {
                title: None,
                commands: ranked,
            }]
        }
    }

    fn show_recent_used_section(&self) -> bool {
        self.state.mode == CommandPanelMode::Root
            && self.state.query.trim().is_empty()
            && !self.state.recent_used_actions.is_empty()
    }

    fn current_commands(&self, config: &Config) -> Vec<CommandPanelCommand> {
        if self.state.mode == CommandPanelMode::RecentList {
            return build_recent_commands(config);
        }
        build_root_commands()
    }

    fn query_hint(&self) -> &'static str {
        if self.state.mode == CommandPanelMode::RecentList {
            "Select recent item"
        } else {
            "Type a command"
        }
    }

    fn footer_hint(&self) -> &'static str {
        if self.state.mode == CommandPanelMode::RecentList {
            "↑ ↓ Navigate   ↵ Open   esc Back"
        } else {
            "↑ ↓ Navigate   ↵ Run   esc Close"
        }
    }

    fn remember_recent_command(&mut self, action: &CommandPanelAction) {
        // `OpenRecentItem` is never rendered by the "Recently used" list (it
        // only comes from the recent-items config), so storing it would just
        // fill the capped list with invisible entries and evict real commands.
        if action_meta(action).is_none() {
            return;
        }
        let key = action_key(action);
        self.state
            .recent_used_actions
            .retain(|item| action_key(item) != key);
        self.state.recent_used_actions.insert(0, action.clone());
        if self.state.recent_used_actions.len() > 5 {
            self.state.recent_used_actions.truncate(5);
        }
    }
}

impl Component for CommandPanel {
    type Message = CustomEvent;
    type Input<'a> = &'a Config;

    fn render(&mut self, ui: &mut egui::Ui, config: Self::Input<'_>) -> Vec<CustomEvent> {
        if !self.state.open {
            return Vec::new();
        }

        let mut events = Vec::new();
        let mut should_close = false;
        let mut pending_action: Option<CommandPanelAction> = None;
        let query_hint = self.query_hint();
        let groups = self.display_commands(config);

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
                        blur: 28,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(110),
                    })
                    .inner_margin(egui::Margin::symmetric(10, 10))
                    .show(ui, |ui| {
                        ui.set_min_width(width);
                        let weak = ui.visuals().weak_text_color();

                        // Search row: icon + field.
                        ui.horizontal(|ui| {
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(MAGNIFYING_GLASS).color(weak));
                            let input = ui.add(
                                egui::TextEdit::singleline(&mut self.state.query)
                                    .id_salt("command_panel_search")
                                    .hint_text(query_hint)
                                    .frame(egui::Frame::NONE)
                                    .desired_width(f32::INFINITY),
                            );
                            input.request_focus();
                        });
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
                                ui.label(
                                    egui::RichText::new(self.footer_hint())
                                        .small()
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
                                    // Keyboard navigation should win over a parked
                                    // pointer, so only adopt a row via hover when
                                    // the pointer actually moved this frame.
                                    let pointer_moved =
                                        ui.input(|i| i.pointer.motion().is_some());
                                    let mut global = 0usize;
                                    for group in &groups {
                                        if let Some(title) = group.title {
                                            group_header(ui, title);
                                        }
                                        for ranked in &group.commands {
                                            let selected = global == self.state.selected;
                                            let response = render_row(ui, ranked, selected);
                                            if response.clicked() {
                                                pending_action =
                                                    Some(ranked.command.action.clone());
                                            }
                                            if response.hovered() && pointer_moved {
                                                self.state.selected = global;
                                            }
                                            if selected {
                                                let rect = response.rect;
                                                let clip = ui.clip_rect();
                                                if rect.top() < clip.top()
                                                    || rect.bottom() > clip.bottom()
                                                {
                                                    ui.scroll_to_rect(rect, None);
                                                }
                                            }
                                            global += 1;
                                        }
                                    }
                                });
                        }

                        // Footer hint bar (with result count on the right).
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(self.footer_hint())
                                    .small()
                                    .color(weak),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{result_count} results"))
                                            .small()
                                            .color(weak),
                                    );
                                },
                            );
                        });
                    });
            });
        let card_rect = card.response.rect;

        // Keyboard navigation (unchanged semantics).
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
                pending_action = Some(flat[self.state.selected].command.action.clone());
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

        if let Some(action) = pending_action {
            match action {
                CommandPanelAction::OpenRecently => {
                    self.remember_recent_command(&action);
                    self.state.mode = CommandPanelMode::RecentList;
                    self.state.query.clear();
                    self.state.selected = 0;
                }
                _ => {
                    self.remember_recent_command(&action);
                    run_command_panel_action(action, &mut events);
                    self.close();
                }
            }
        }

        events
    }
}

/// One result row: icon column plus a two-line title/subtitle. The row is
/// painted manually (mirroring the file tree) so selection, emphasis and the
/// accent edge stay under our control.
fn render_row(ui: &mut egui::Ui, ranked: &RankedCommand, selected: bool) -> egui::Response {
    let row_height = ROW_HEIGHT;
    let width = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, row_height), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    if selected {
        ui.painter()
            .rect_filled(rect, 0.0, ui.visuals().selection.bg_fill);
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(2.0, row_height)),
            0.0,
            ui.visuals().hyperlink_color,
        );
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
    let title_color = if selected {
        strong
    } else {
        ui.visuals().text_color()
    };
    let job = title_layout_job(
        &ranked.command.title,
        ranked.match_spans.as_deref(),
        body_font.clone(),
        title_color,
        accent,
    );
    let galley = ui.painter().layout_job(job);
    let title_top = rect.top() + 5.0;
    let title_bottom = title_top + galley.size().y;

    // Icon column: vertically centered on the first (title) line.
    let icon_center =
        egui::pos2(rect.left() + 6.0 + 12.0, title_top + galley.size().y * 0.5);
    match &ranked.command.icon {
        CommandIcon::Phosphor(glyph) => {
            let color = if selected { accent } else { weak };
            ui.painter().text(
                icon_center,
                egui::Align2::CENTER_CENTER,
                *glyph,
                body_font.clone(),
                color,
            );
        }
        CommandIcon::Folder => {
            let color = if selected { accent } else { weak };
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

    ui.painter().galley(egui::pos2(text_left, title_top), galley.clone(), title_color);

    if let Some(subtitle) = &ranked.command.subtitle {
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

    response
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

/// Layout the title line, highlighting matched characters in `accent`.
fn title_layout_job(
    title: &str,
    spans: Option<&[(usize, usize)]>,
    font: egui::FontId,
    base_color: egui::Color32,
    accent: egui::Color32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.halign = egui::Align::LEFT;
    let chars: Vec<char> = title.chars().collect();
    let is_matched =
        |i: usize| spans.is_some_and(|spans| spans.iter().any(|&(start, end)| i >= start && i < end));

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
    job.append(
        &segment,
        0.0,
        if run_matched {
            highlighted
        } else {
            base
        },
    );
    job
}

fn run_command_panel_action(action: CommandPanelAction, events: &mut Vec<CustomEvent>) {
    match action {
        CommandPanelAction::SaveFile => {
            events.push(CustomEvent::Document(DocumentEvent::SaveFile));
        }
        CommandPanelAction::FormatFile => {
            events.push(CustomEvent::Document(DocumentEvent::FormatFile));
        }
        CommandPanelAction::OpenFile => {
            if let Some(path) = rfd::FileDialog::new().pick_file() {
                events.push(CustomEvent::App(AppEvent::OpenFile(path)));
            }
        }
        CommandPanelAction::OpenFolder => {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                events.push(CustomEvent::App(AppEvent::OpenFolder(path)));
            }
        }
        CommandPanelAction::SwitchToConfiguration => {
            events.push(CustomEvent::App(AppEvent::OpenConfiguration));
        }
        CommandPanelAction::ToggleSidebar => {
            events.push(CustomEvent::Shell(ShellEvent::ToggleSidebar));
        }
        CommandPanelAction::ClearRecentItems => {
            events.push(CustomEvent::App(AppEvent::ClearRecentItems));
        }
        CommandPanelAction::OpenRecentItem { path, is_dir } => {
            if is_dir {
                if path.is_dir() {
                    events.push(CustomEvent::App(AppEvent::OpenFolder(path)));
                }
            } else if path.is_file() {
                events.push(CustomEvent::App(AppEvent::OpenFile(path)));
            }
        }
        CommandPanelAction::OpenRecently => {}
    }
}

/// Title/subtitle/icon for a non-recent-item action. `None` for
/// [`CommandPanelAction::OpenRecentItem`], which is never used by the command
/// lists (it only comes from the recent-items config).
fn action_meta(action: &CommandPanelAction) -> Option<(&'static str, Option<&'static str>, CommandIcon)> {
    use CommandIcon::Phosphor;
    match action {
        CommandPanelAction::SaveFile => {
            Some(("Save File", Some("File"), Phosphor(FLOPPY_DISK)))
        }
        CommandPanelAction::FormatFile => {
            Some(("Format File", Some("File"), Phosphor(TEXT_ALIGN_LEFT)))
        }
        CommandPanelAction::OpenFile => Some(("Open File", Some("File"), Phosphor(FILES))),
        CommandPanelAction::OpenRecently => {
            Some(("Open Recently", Some("File"), Phosphor(CLOCK_COUNTER_CLOCKWISE)))
        }
        CommandPanelAction::OpenFolder => {
            Some(("Open Folder", Some("File"), Phosphor(FOLDER_SIMPLE)))
        }
        CommandPanelAction::SwitchToConfiguration => {
            Some(("Switch To Configuration", Some("View"), Phosphor(GEAR_SIX)))
        }
        CommandPanelAction::ToggleSidebar => {
            Some(("Toggle Sidebar", Some("View"), Phosphor(SIDEBAR_SIMPLE)))
        }
        CommandPanelAction::ClearRecentItems => {
            Some(("Clear Recent Items", Some("Workspace"), Phosphor(TRASH)))
        }
        CommandPanelAction::OpenRecentItem { .. } => None,
    }
}

fn build_root_commands() -> Vec<CommandPanelCommand> {
    let defs: [(CommandPanelAction, &[&str]); 8] = [
        (
            CommandPanelAction::SaveFile,
            &["save", "write", "file", "persist"],
        ),
        (
            CommandPanelAction::FormatFile,
            &["format", "formatter", "style", "pretty"],
        ),
        (CommandPanelAction::OpenFile, &["file", "open", "load"]),
        (
            CommandPanelAction::OpenRecently,
            &["recent", "recently", "history", "open"],
        ),
        (
            CommandPanelAction::OpenFolder,
            &["folder", "workspace", "open"],
        ),
        (
            CommandPanelAction::SwitchToConfiguration,
            &["config", "settings", "preferences", "view"],
        ),
        (
            CommandPanelAction::ToggleSidebar,
            &["sidebar", "panel", "tree", "toggle"],
        ),
        (
            CommandPanelAction::ClearRecentItems,
            &["recent", "clear", "history"],
        ),
    ];

    defs.into_iter()
        .filter_map(|(action, keywords)| {
            let (title, subtitle, icon) = action_meta(&action)?;
            Some(CommandPanelCommand {
                title: title.to_string(),
                subtitle: subtitle.map(str::to_string),
                keywords: keywords.iter().map(|keyword| keyword.to_string()).collect(),
                icon,
                action,
            })
        })
        .collect()
}

fn build_recent_commands(config: &crate::settings::Config) -> Vec<CommandPanelCommand> {
    config
        .recent_items
        .iter()
        .map(|item| {
            let is_dir = item.is_dir;
            let path = &item.path;
            let path_display = path.display().to_string();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();
            let kind = if is_dir { "Folder" } else { "File" };
            let title = if name.is_empty() {
                path_display.clone()
            } else {
                name.clone()
            };
            let icon = if is_dir {
                CommandIcon::Folder
            } else {
                CommandIcon::File(path.clone())
            };
            CommandPanelCommand {
                title,
                subtitle: Some(path_display.clone()),
                keywords: vec![
                    "recent".to_string(),
                    "open".to_string(),
                    "recently".to_string(),
                    kind.to_ascii_lowercase(),
                    name,
                    path_display,
                ],
                icon,
                action: CommandPanelAction::OpenRecentItem {
                    path: path.clone(),
                    is_dir,
                },
            }
        })
        .collect()
}

fn build_recent_used_commands(actions: &[CommandPanelAction]) -> Vec<RankedCommand> {
    actions
        .iter()
        .filter_map(|action| {
            let (title, subtitle, icon) = action_meta(action)?;
            Some(RankedCommand {
                command: CommandPanelCommand {
                    title: title.to_string(),
                    subtitle: subtitle.map(str::to_string),
                    keywords: vec![],
                    icon,
                    action: action.clone(),
                },
                score: i32::MAX,
                match_spans: None,
            })
        })
        .collect()
}

fn action_key(action: &CommandPanelAction) -> String {
    match action {
        CommandPanelAction::SaveFile => "save-file".to_string(),
        CommandPanelAction::FormatFile => "format-file".to_string(),
        CommandPanelAction::OpenFile => "open-file".to_string(),
        CommandPanelAction::OpenRecently => "open-recently".to_string(),
        CommandPanelAction::OpenFolder => "open-folder".to_string(),
        CommandPanelAction::SwitchToConfiguration => "switch-configuration".to_string(),
        CommandPanelAction::ToggleSidebar => "toggle-sidebar".to_string(),
        CommandPanelAction::ClearRecentItems => "clear-recent-items".to_string(),
        CommandPanelAction::OpenRecentItem { path, is_dir } => {
            format!(
                "open-recent-item-{}-{}",
                if *is_dir { "dir" } else { "file" },
                path.display()
            )
        }
    }
}

fn rank_commands(query: &str, commands: Vec<CommandPanelCommand>) -> Vec<RankedCommand> {
    let mut ranked = commands
        .into_iter()
        .filter_map(|command| {
            score_command(query, &command).map(|(score, match_spans)| RankedCommand {
                command,
                score,
                match_spans,
            })
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.command.title.cmp(&right.command.title))
    });
    ranked
}

fn score_command(
    query: &str,
    command: &CommandPanelCommand,
) -> Option<(i32, Option<Vec<(usize, usize)>>)> {
    let normalized = query.trim();
    if normalized.is_empty() {
        return Some((0, None));
    }

    let title_match = fuzzy_match(normalized, &command.title);
    let mut best = title_match.as_ref().map(|matched| matched.score);
    for keyword in &command.keywords {
        if let Some(matched) = fuzzy_match(normalized, keyword)
            && best.is_none_or(|score| matched.score > score)
        {
            best = Some(matched.score);
        }
    }
    let score = best?;

    // Emphasize only when the query is a subsequence of the title itself and
    // lowercasing did not change its length (so char indices still align).
    let match_spans = if let Some(matched) = title_match {
        if command.title.to_lowercase().chars().count() == command.title.chars().count() {
            Some(positions_to_ranges(&matched.positions))
        } else {
            None
        }
    } else {
        None
    };

    Some((score, match_spans))
}

/// A fuzzy (subsequence) match with its score and matched char positions.
struct FuzzyMatch {
    score: i32,
    positions: Vec<usize>,
}

fn fuzzy_match(query: &str, candidate: &str) -> Option<FuzzyMatch> {
    let query = query.to_lowercase();
    let candidate = candidate.to_lowercase();
    let candidate_chars = candidate.chars().collect::<Vec<_>>();

    let mut positions = Vec::with_capacity(query.len());
    let mut search_index = 0usize;
    for q in query.chars() {
        let mut found = None;
        for (idx, c) in candidate_chars.iter().enumerate().skip(search_index) {
            if *c == q {
                found = Some(idx);
                search_index = idx + 1;
                break;
            }
        }
        positions.push(found?);
    }

    let first = *positions.first()? as i32;
    let last = *positions.last()? as i32;
    let span = last - first + 1;
    let compactness_bonus = (query.chars().count() as i32) * 16 - span * 4;
    let prefix_bonus = if first == 0 { 24 } else { 0 };
    let length_penalty = candidate_chars.len() as i32;

    Some(FuzzyMatch {
        score: compactness_bonus + prefix_bonus - first - length_penalty,
        positions,
    })
}

/// Score-only wrapper kept for the tests and simpler call sites.
#[cfg(test)]
fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    fuzzy_match(query, candidate).map(|matched| matched.score)
}

/// Collapse consecutive matched positions into inclusive char ranges.
fn positions_to_ranges(positions: &[usize]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut iter = positions.iter().copied();
    if let Some(first) = iter.next() {
        let mut start = first;
        let mut end = first + 1;
        for position in iter {
            if position == end {
                end = position + 1;
            } else {
                ranges.push((start, end));
                start = position;
                end = position + 1;
            }
        }
        ranges.push((start, end));
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::{
        action_key, build_recent_commands, build_recent_used_commands, build_root_commands,
        fuzzy_match, fuzzy_score, positions_to_ranges, rank_commands, score_command,
        CommandIcon, CommandPanelAction, CommandPanelCommand, RankedCommand,
    };
    use crate::settings::types::RecentItem;
    use std::path::PathBuf;

    #[test]
    fn fuzzy_score_prefers_compact_matches() {
        let compact = fuzzy_score("opf", "Open File").unwrap();
        let sparse = fuzzy_score("opf", "Open Folder").unwrap();
        assert!(compact > sparse);
    }

    #[test]
    fn fuzzy_score_requires_subsequence() {
        assert!(fuzzy_score("xyz", "Open File").is_none());
    }

    #[test]
    fn root_commands_include_open_recently() {
        let has_open_recently = build_root_commands()
            .into_iter()
            .any(|command| command.title == "Open Recently");
        assert!(has_open_recently);
    }

    #[test]
    fn recent_list_builds_from_recent_items() {
        let mut config = crate::settings::Config::default();
        config.recent_items = vec![RecentItem {
            path: PathBuf::from("/tmp/rustfmt.toml"),
            is_dir: false,
        }];
        let commands = build_recent_commands(&config);
        assert_eq!(commands.len(), 1);
        // The row leads with the file name and shows the path as a subtitle.
        assert_eq!(commands[0].title, "rustfmt.toml");
        assert_eq!(commands[0].subtitle.as_deref(), Some("/tmp/rustfmt.toml"));
        assert!(matches!(&commands[0].icon, CommandIcon::File(_)));
    }

    #[test]
    fn recent_folders_get_folder_icon() {
        let mut config = crate::settings::Config::default();
        config.recent_items = vec![RecentItem {
            path: PathBuf::from("/tmp/project"),
            is_dir: true,
        }];
        let commands = build_recent_commands(&config);
        assert!(matches!(&commands[0].icon, CommandIcon::Folder));
    }

    #[test]
    fn rank_commands_can_match_recent_list_entry() {
        let ranked = rank_commands(
            "rustfmt",
            vec![CommandPanelCommand {
                title: "rustfmt.toml".to_string(),
                subtitle: Some("/tmp/rustfmt.toml".to_string()),
                keywords: vec!["recent".to_string(), "rustfmt.toml".to_string()],
                icon: CommandIcon::File(PathBuf::from("/tmp/rustfmt.toml")),
                action: CommandPanelAction::OpenRecentItem {
                    path: PathBuf::from("/tmp/rustfmt.toml"),
                    is_dir: false,
                },
            }],
        );
        assert_eq!(ranked[0].command.title, "rustfmt.toml");
    }

    #[test]
    fn recent_used_commands_keep_latest_first() {
        let commands = build_recent_used_commands(&[
            CommandPanelAction::OpenFolder,
            CommandPanelAction::OpenFile,
        ]);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command.title, "Open Folder");
        assert_eq!(commands[1].command.title, "Open File");
        // Recent-used rows carry icons and categories.
        assert!(matches!(
            &commands[0].command.icon,
            CommandIcon::Phosphor(_)
        ));
        assert_eq!(commands[0].command.subtitle.as_deref(), Some("File"));
    }

    #[test]
    fn action_key_differs_for_recent_file_and_folder() {
        let file_key = action_key(&CommandPanelAction::OpenRecentItem {
            path: PathBuf::from("/tmp/project"),
            is_dir: false,
        });
        let folder_key = action_key(&CommandPanelAction::OpenRecentItem {
            path: PathBuf::from("/tmp/project"),
            is_dir: true,
        });
        assert_ne!(file_key, folder_key);
    }

    #[test]
    fn recent_used_skips_open_recent_item_actions() {
        use crate::chrome::ui::CommandPanel;
        let mut panel = CommandPanel::default();
        // Opening a recent item must not pollute the capped "Recently used" list.
        panel.remember_recent_command(&CommandPanelAction::OpenRecentItem {
            path: PathBuf::from("/tmp/project"),
            is_dir: true,
        });
        assert!(panel.state.recent_used_actions.is_empty());
        // A real command still records.
        panel.remember_recent_command(&CommandPanelAction::SaveFile);
        assert_eq!(panel.state.recent_used_actions.len(), 1);
    }

    #[test]
    fn action_meta_consistency_between_root_and_recent_used() {
        use CommandPanelAction::*;
        let actions = [
            SaveFile, FormatFile, OpenFile, OpenRecently, OpenFolder, SwitchToConfiguration,
            ToggleSidebar, ClearRecentItems,
        ];
        let root_titles: Vec<String> =
            build_root_commands().into_iter().map(|c| c.title).collect();
        for action in actions {
            let recent_titles: Vec<String> = build_recent_used_commands(&[action.clone()])
                .into_iter()
                .map(|r: RankedCommand| r.command.title)
                .collect();
            // The recent-used title must match one of the root command titles.
            assert!(
                root_titles.contains(&recent_titles[0]),
                "meta drift for {action:?}: {}",
                recent_titles[0]
            );
        }
    }

    #[test]
    fn score_command_produces_title_match_spans() {
        let command = CommandPanelCommand {
            title: "Save File".to_string(),
            subtitle: None,
            keywords: vec![],
            icon: CommandIcon::Phosphor("x"),
            action: CommandPanelAction::SaveFile,
        };
        // "sf" matches "Save File" at chars 0 and 5 -> two single-char ranges.
        let (score, spans) = score_command("sf", &command).unwrap();
        assert!(score > 0);
        assert_eq!(spans.unwrap(), vec![(0, 1), (5, 6)]);
    }

    #[test]
    fn score_command_skips_emphasis_for_keyword_only_match() {
        let command = CommandPanelCommand {
            title: "Save File".to_string(),
            subtitle: None,
            keywords: vec!["persist".to_string()],
            icon: CommandIcon::Phosphor("x"),
            action: CommandPanelAction::SaveFile,
        };
        // "persist" matches the keyword but not the title -> no emphasis.
        let (_score, spans) = score_command("persist", &command).unwrap();
        assert!(spans.is_none());
    }

    #[test]
    fn empty_query_has_no_emphasis() {
        let command = CommandPanelCommand {
            title: "Save File".to_string(),
            subtitle: None,
            keywords: vec![],
            icon: CommandIcon::Phosphor("x"),
            action: CommandPanelAction::SaveFile,
        };
        let (_score, spans) = score_command("", &command).unwrap();
        assert!(spans.is_none());
    }

    #[test]
    fn positions_to_ranges_merges_runs() {
        assert_eq!(positions_to_ranges(&[0, 1, 2, 5]), vec![(0, 3), (5, 6)]);
        assert_eq!(positions_to_ranges(&[3]), vec![(3, 4)]);
        assert_eq!(positions_to_ranges(&[]), vec![]);
    }

    #[test]
    fn fuzzy_match_returns_positions() {
        // "opf" matches "open file" at chars 0, 1 and 5.
        let matched = fuzzy_match("opf", "Open File").unwrap();
        assert_eq!(matched.positions, vec![0, 1, 5]);
    }
}
