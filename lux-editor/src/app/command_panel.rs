use super::{App, ShellView};
use eframe::egui;
use std::path::PathBuf;

#[derive(Clone, Copy, Default, Eq, PartialEq)]
enum CommandPanelMode {
    #[default]
    Root,
    RecentList,
}

#[derive(Default)]
pub(super) struct CommandPanelState {
    pub(super) open: bool,
    pub(super) query: String,
    pub(super) selected: usize,
    mode: CommandPanelMode,
    recent_used_actions: Vec<CommandPanelAction>,
}

#[derive(Clone)]
enum CommandPanelAction {
    OpenFile,
    OpenRecently,
    OpenFolder,
    SwitchToEditor,
    SwitchToConfiguration,
    ClearRecentItems,
    OpenRecentItem { path: PathBuf, is_dir: bool },
}

#[derive(Clone)]
struct CommandPanelCommand {
    title: String,
    keywords: Vec<String>,
    action: CommandPanelAction,
}

#[derive(Clone)]
struct RankedCommand {
    command: CommandPanelCommand,
    score: i32,
}

impl App {
    pub(super) fn toggle_command_panel(&mut self) {
        if self.command_panel.open {
            self.close_command_panel();
            return;
        }
        self.command_panel.open = true;
        self.command_panel.query.clear();
        self.command_panel.selected = 0;
        self.command_panel.mode = CommandPanelMode::Root;
    }

    pub(super) fn command_panel_open(&self) -> bool {
        self.command_panel.open
    }

    fn close_command_panel(&mut self) {
        self.command_panel.open = false;
        self.command_panel.query.clear();
        self.command_panel.selected = 0;
        self.command_panel.mode = CommandPanelMode::Root;
    }

    pub(super) fn render_command_panel(&mut self, ctx: &egui::Context) {
        if !self.command_panel.open {
            return;
        }

        let mut should_close = false;
        let mut pending_action: Option<CommandPanelAction> = None;
        let query_hint = self.query_hint();
        let display_commands = self.display_commands();

        egui::Window::new("Command Panel")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 40.0))
            .default_width(520.0)
            .show(ctx, |ui| {
                let input = ui.add(
                    egui::TextEdit::singleline(&mut self.command_panel.query).hint_text(query_hint),
                );
                input.request_focus();

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    if self.command_panel.mode == CommandPanelMode::RecentList {
                        self.open_root_commands();
                    } else {
                        should_close = true;
                    }
                }

                if display_commands.is_empty() {
                    self.command_panel.selected = 0;
                    ui.add_space(8.0);
                    ui.label("No matching commands");
                    return;
                }

                if self.command_panel.selected >= display_commands.len() {
                    self.command_panel.selected = display_commands.len().saturating_sub(1);
                }

                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    self.command_panel.selected =
                        (self.command_panel.selected + 1) % display_commands.len();
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    self.command_panel.selected = if self.command_panel.selected == 0 {
                        display_commands.len().saturating_sub(1)
                    } else {
                        self.command_panel.selected.saturating_sub(1)
                    };
                }
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    pending_action = Some(
                        display_commands[self.command_panel.selected]
                            .command
                            .action
                            .clone(),
                    );
                }

                ui.add_space(8.0);
                if self.show_recent_used_section() {
                    ui.label("Recent Used");
                    ui.separator();
                    ui.add_space(4.0);
                }
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        for (index, ranked) in display_commands.iter().enumerate() {
                            let selected = index == self.command_panel.selected;
                            let response = ui.selectable_label(selected, &ranked.command.title);
                            if response.clicked() {
                                pending_action = Some(ranked.command.action.clone());
                            }
                            if response.hovered() {
                                self.command_panel.selected = index;
                            }
                        }
                    });
            });

        if should_close {
            self.close_command_panel();
            return;
        }

        if let Some(action) = pending_action {
            match action {
                CommandPanelAction::OpenRecently => {
                    self.remember_recent_command(&action);
                    self.command_panel.mode = CommandPanelMode::RecentList;
                    self.command_panel.query.clear();
                    self.command_panel.selected = 0;
                }
                _ => {
                    self.remember_recent_command(&action);
                    self.run_command_panel_action(action, ctx);
                    self.close_command_panel();
                }
            }
        }
    }

    fn display_commands(&self) -> Vec<RankedCommand> {
        let ranked_commands = rank_commands(&self.command_panel.query, self.current_commands());
        if !self.show_recent_used_section() {
            return ranked_commands;
        }

        let mut recent_commands =
            build_recent_used_commands(&self.command_panel.recent_used_actions);
        let recent_keys = recent_commands
            .iter()
            .map(|command| action_key(&command.command.action))
            .collect::<Vec<_>>();
        let mut combined = Vec::with_capacity(recent_commands.len() + ranked_commands.len());
        combined.append(&mut recent_commands);
        for ranked in ranked_commands {
            let key = action_key(&ranked.command.action);
            if !recent_keys.iter().any(|recent_key| recent_key == &key) {
                combined.push(ranked);
            }
        }
        combined
    }

    fn show_recent_used_section(&self) -> bool {
        self.command_panel.mode == CommandPanelMode::Root
            && self.command_panel.query.trim().is_empty()
            && !self.command_panel.recent_used_actions.is_empty()
    }

    fn current_commands(&self) -> Vec<CommandPanelCommand> {
        if self.command_panel.mode == CommandPanelMode::RecentList {
            return build_recent_commands(&self.editor_config);
        }
        build_root_commands()
    }

    fn query_hint(&self) -> &'static str {
        if self.command_panel.mode == CommandPanelMode::RecentList {
            "Select recent item"
        } else {
            "Type a command"
        }
    }

    fn open_root_commands(&mut self) {
        self.command_panel.mode = CommandPanelMode::Root;
        self.command_panel.query.clear();
        self.command_panel.selected = 0;
    }

    fn remember_recent_command(&mut self, action: &CommandPanelAction) {
        let key = action_key(action);
        self.command_panel
            .recent_used_actions
            .retain(|item| action_key(item) != key);
        self.command_panel
            .recent_used_actions
            .insert(0, action.clone());
        if self.command_panel.recent_used_actions.len() > 5 {
            self.command_panel.recent_used_actions.truncate(5);
        }
    }

    fn run_command_panel_action(&mut self, action: CommandPanelAction, ctx: &egui::Context) {
        match action {
            CommandPanelAction::OpenFile => {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    self.open_file(path, ctx);
                }
            }
            CommandPanelAction::OpenRecently => {}
            CommandPanelAction::OpenFolder => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.open_folder(path);
                }
            }
            CommandPanelAction::SwitchToEditor => self.shell_view = ShellView::Editor,
            CommandPanelAction::SwitchToConfiguration => {
                self.shell_view = ShellView::Configuration;
            }
            CommandPanelAction::ClearRecentItems => self.editor_config.clear_recent_items(),
            CommandPanelAction::OpenRecentItem { path, is_dir } => {
                if is_dir {
                    if path.is_dir() {
                        self.open_folder(path);
                    }
                } else if path.is_file() {
                    self.open_file(path, ctx);
                }
            }
        }
    }
}

fn build_root_commands() -> Vec<CommandPanelCommand> {
    vec![
        CommandPanelCommand {
            title: "Open File".to_string(),
            keywords: vec!["file".to_string(), "open".to_string(), "load".to_string()],
            action: CommandPanelAction::OpenFile,
        },
        CommandPanelCommand {
            title: "Open Recently".to_string(),
            keywords: vec![
                "recent".to_string(),
                "recently".to_string(),
                "history".to_string(),
                "open".to_string(),
            ],
            action: CommandPanelAction::OpenRecently,
        },
        CommandPanelCommand {
            title: "Open Folder".to_string(),
            keywords: vec![
                "folder".to_string(),
                "workspace".to_string(),
                "open".to_string(),
            ],
            action: CommandPanelAction::OpenFolder,
        },
        CommandPanelCommand {
            title: "Switch To Editor".to_string(),
            keywords: vec!["editor".to_string(), "view".to_string(), "mode".to_string()],
            action: CommandPanelAction::SwitchToEditor,
        },
        CommandPanelCommand {
            title: "Switch To Configuration".to_string(),
            keywords: vec![
                "config".to_string(),
                "settings".to_string(),
                "preferences".to_string(),
                "view".to_string(),
            ],
            action: CommandPanelAction::SwitchToConfiguration,
        },
        CommandPanelCommand {
            title: "Clear Recent Items".to_string(),
            keywords: vec![
                "recent".to_string(),
                "clear".to_string(),
                "history".to_string(),
            ],
            action: CommandPanelAction::ClearRecentItems,
        },
    ]
}

fn build_recent_commands(config: &crate::config::Config) -> Vec<CommandPanelCommand> {
    let mut commands = Vec::new();
    for item in &config.recent_items {
        let kind = if item.is_dir { "Folder" } else { "File" };
        let path_display = item.path.display().to_string();
        let file_name = item
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        commands.push(CommandPanelCommand {
            title: format!("{}: {}", kind, path_display),
            keywords: vec![
                "recent".to_string(),
                "open".to_string(),
                "recently".to_string(),
                kind.to_ascii_lowercase(),
                file_name,
                path_display,
            ],
            action: CommandPanelAction::OpenRecentItem {
                path: item.path.clone(),
                is_dir: item.is_dir,
            },
        });
    }

    commands
}

fn build_recent_used_commands(actions: &[CommandPanelAction]) -> Vec<RankedCommand> {
    actions
        .iter()
        .map(|action| RankedCommand {
            command: CommandPanelCommand {
                title: recent_used_title(action),
                keywords: vec![],
                action: action.clone(),
            },
            score: i32::MAX,
        })
        .collect()
}

fn recent_used_title(action: &CommandPanelAction) -> String {
    match action {
        CommandPanelAction::OpenFile => "Open File".to_string(),
        CommandPanelAction::OpenRecently => "Open Recently".to_string(),
        CommandPanelAction::OpenFolder => "Open Folder".to_string(),
        CommandPanelAction::SwitchToEditor => "Switch To Editor".to_string(),
        CommandPanelAction::SwitchToConfiguration => "Switch To Configuration".to_string(),
        CommandPanelAction::ClearRecentItems => "Clear Recent Items".to_string(),
        CommandPanelAction::OpenRecentItem { path, is_dir } => {
            let kind = if *is_dir { "Folder" } else { "File" };
            format!("{}: {}", kind, path.display())
        }
    }
}

fn action_key(action: &CommandPanelAction) -> String {
    match action {
        CommandPanelAction::OpenFile => "open-file".to_string(),
        CommandPanelAction::OpenRecently => "open-recently".to_string(),
        CommandPanelAction::OpenFolder => "open-folder".to_string(),
        CommandPanelAction::SwitchToEditor => "switch-editor".to_string(),
        CommandPanelAction::SwitchToConfiguration => "switch-configuration".to_string(),
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
            score_command(query, &command).map(|score| RankedCommand { command, score })
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

fn score_command(query: &str, command: &CommandPanelCommand) -> Option<i32> {
    let normalized = query.trim();
    if normalized.is_empty() {
        return Some(0);
    }

    let mut best = fuzzy_score(normalized, &command.title)?;
    for keyword in &command.keywords {
        if let Some(score) = fuzzy_score(normalized, keyword)
            && score > best
        {
            best = score;
        }
    }
    Some(best)
}

fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
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

    Some(compactness_bonus + prefix_bonus - first - length_penalty)
}

#[cfg(test)]
mod tests {
    use super::{
        CommandPanelAction, CommandPanelCommand, action_key, build_recent_commands,
        build_recent_used_commands, build_root_commands, fuzzy_score, rank_commands,
    };
    use crate::config::RecentItem;
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
        let mut config = crate::config::Config::default();
        config.recent_items = vec![RecentItem {
            path: PathBuf::from("/tmp/rustfmt.toml"),
            is_dir: false,
        }];
        let commands = build_recent_commands(&config);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].title, "File: /tmp/rustfmt.toml");
    }

    #[test]
    fn rank_commands_can_match_recent_list_entry() {
        let ranked = rank_commands(
            "rustfmt",
            vec![CommandPanelCommand {
                title: "File: /tmp/rustfmt.toml".to_string(),
                keywords: vec!["recent".to_string(), "rustfmt.toml".to_string()],
                action: CommandPanelAction::OpenRecentItem {
                    path: PathBuf::from("/tmp/rustfmt.toml"),
                    is_dir: false,
                },
            }],
        );
        assert_eq!(ranked[0].command.title, "File: /tmp/rustfmt.toml");
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
}
