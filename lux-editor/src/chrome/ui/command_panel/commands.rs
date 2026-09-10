//! Palette command registry. A command is one plain-data entry in [`COMMANDS`]:
//! id, title, category, keywords, icon and a pure runner that pushes the
//! `CustomEvent`s the app's action layer executes. Adding a command means
//! appending one `Command` below — no other wiring. A command that needs new
//! app behavior first adds a domain event variant + a `Ctx` action arm (see
//! `events.rs` / `app/actions`), the same cost keybindings and menus pay.

use crate::events::{AppEvent, CustomEvent, DocumentEvent, ShellEvent};
use crate::settings::Config;
use egui_phosphor::regular::{
    CLOCK_COUNTER_CLOCKWISE, EYE, FILES, FLOPPY_DISK, FOLDER_SIMPLE, GEAR_SIX, SIDEBAR_SIMPLE,
    TEXT_ALIGN_LEFT, TRASH,
};
use std::path::PathBuf;
use std::sync::LazyLock;

/// Frame context commands can gate their visibility on.
#[derive(Clone, Copy)]
pub(crate) struct PaletteContext {
    /// The active tab is a markdown document.
    pub active_is_markdown: bool,
}

/// One registered palette command.
pub(crate) struct Command {
    /// Stable identity; also the "Recently used" dedup key.
    pub id: &'static str,
    pub title: &'static str,
    /// Row subtitle and grouping label (e.g. "File", "View", "Workspace").
    pub category: &'static str,
    pub keywords: &'static [&'static str],
    /// egui-phosphor glyph.
    pub icon: &'static str,
    pub kind: CommandKind,
    /// Whether the command is listed this frame.
    pub available: fn(&PaletteContext) -> bool,
    /// Push the command's effect(s); the host forwards them to the app.
    pub run: fn(&mut Vec<CustomEvent>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommandKind {
    RunAndClose,
    /// Leave the palette open on the recents sub-list instead of running.
    ShowRecents,
}

/// Which glyph a row leads with: a phosphor glyph, a folder, or a themed
/// devicon for a specific file path.
#[derive(Clone)]
pub(crate) enum CommandIcon {
    Phosphor(&'static str),
    Folder,
    File(PathBuf),
}

/// What selecting a row should do.
#[derive(Clone)]
pub(crate) enum PaletteTarget {
    /// Run a registered command.
    Registered(&'static Command),
    /// Open a recent item; the path is re-checked at run time.
    OpenRecent { path: PathBuf, is_dir: bool },
}

/// A row ready for ranking/rendering.
#[derive(Clone)]
pub(crate) struct PaletteItem {
    pub title: String,
    pub subtitle: Option<String>,
    pub keywords: Vec<String>,
    pub icon: CommandIcon,
    pub target: PaletteTarget,
}

/// The full palette command list. Add a command by appending one `Command`
/// entry here; keep ids unique.
pub(crate) static COMMANDS: LazyLock<Vec<Command>> = LazyLock::new(|| {
    vec![
        Command {
            id: "save-file",
            title: "Save File",
            category: "File",
            keywords: &["save", "write", "file", "persist"],
            icon: FLOPPY_DISK,
            kind: CommandKind::RunAndClose,
            available: |_| true,
            run: |events| events.push(CustomEvent::Document(DocumentEvent::SaveFile)),
        },
        Command {
            id: "format-file",
            title: "Format File",
            category: "File",
            keywords: &["format", "formatter", "style", "pretty"],
            icon: TEXT_ALIGN_LEFT,
            kind: CommandKind::RunAndClose,
            available: |_| true,
            run: |events| events.push(CustomEvent::Document(DocumentEvent::FormatFile)),
        },
        Command {
            id: "open-file",
            title: "Open File",
            category: "File",
            keywords: &["file", "open", "load"],
            icon: FILES,
            kind: CommandKind::RunAndClose,
            available: |_| true,
            run: |events| {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    events.push(CustomEvent::App(AppEvent::OpenFile(path)));
                }
            },
        },
        Command {
            id: "open-recently",
            title: "Open Recently",
            category: "File",
            keywords: &["recent", "recently", "history", "open"],
            icon: CLOCK_COUNTER_CLOCKWISE,
            kind: CommandKind::ShowRecents,
            available: |_| true,
            run: |_| {},
        },
        Command {
            id: "open-folder",
            title: "Open Folder",
            category: "File",
            keywords: &["folder", "workspace", "open"],
            icon: FOLDER_SIMPLE,
            kind: CommandKind::RunAndClose,
            available: |_| true,
            run: |events| {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    events.push(CustomEvent::App(AppEvent::OpenFolder(path)));
                }
            },
        },
        Command {
            id: "switch-to-configuration",
            title: "Switch To Configuration",
            category: "View",
            keywords: &["config", "settings", "preferences", "view"],
            icon: GEAR_SIX,
            kind: CommandKind::RunAndClose,
            available: |_| true,
            run: |events| events.push(CustomEvent::App(AppEvent::OpenConfiguration)),
        },
        Command {
            id: "toggle-sidebar",
            title: "Toggle Sidebar",
            category: "View",
            keywords: &["sidebar", "panel", "tree", "toggle"],
            icon: SIDEBAR_SIMPLE,
            kind: CommandKind::RunAndClose,
            available: |_| true,
            run: |events| events.push(CustomEvent::Shell(ShellEvent::ToggleSidebar)),
        },
        Command {
            id: "toggle-markdown-preview",
            title: "Toggle Markdown Preview",
            category: "View",
            keywords: &["markdown", "preview", "toggle", "view", "render"],
            icon: EYE,
            kind: CommandKind::RunAndClose,
            available: |context| context.active_is_markdown,
            run: |events| {
                events.push(CustomEvent::Shell(ShellEvent::ToggleMarkdownPreview));
            }
        },
        Command {
            id: "clear-recent-items",
            title: "Clear Recent Items",
            category: "Workspace",
            keywords: &["recent", "clear", "history"],
            icon: TRASH,
            kind: CommandKind::RunAndClose,
            available: |_| true,
            run: |events| events.push(CustomEvent::App(AppEvent::ClearRecentItems)),
        },
    ]
});

impl Command {
    pub(crate) fn palette_item(self: &'static Command) -> PaletteItem {
        PaletteItem {
            title: self.title.to_string(),
            subtitle: Some(self.category.to_string()),
            keywords: self.keywords.iter().map(|keyword| keyword.to_string()).collect(),
            icon: CommandIcon::Phosphor(self.icon),
            target: PaletteTarget::Registered(self),
        }
    }
}

pub(crate) fn by_id(id: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.id == id)
}

/// "Recently used" rows: stored ids resolve back through the registry. Dynamic
/// recents are never stored, so every id here resolves to a static command.
pub(crate) fn items_from_ids(
    ids: &[&'static str],
    context: &PaletteContext,
) -> Vec<PaletteItem> {
    ids.iter()
        .filter_map(|id| by_id(id))
        .filter(|command| (command.available)(context))
        .map(Command::palette_item)
        .collect()
}

/// Dynamic recent-item rows, built from the config's recents list.
pub(crate) fn recent_items(config: &Config) -> Vec<PaletteItem> {
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
            PaletteItem {
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
                target: PaletteTarget::OpenRecent {
                    path: path.clone(),
                    is_dir,
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{recent_items, Command, CommandIcon, CommandKind, PaletteContext, PaletteTarget};
    use crate::settings::types::RecentItem;
    use std::path::PathBuf;

    #[test]
    fn registry_has_unique_ids() {
        let ids: Vec<&str> = super::COMMANDS.iter().map(|c| c.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "duplicate command id: {ids:?}");
        assert!(!ids.is_empty());
    }

    #[test]
    fn open_recently_is_a_show_recents_command() {
        let command = super::by_id("open-recently").unwrap();
        assert_eq!(command.kind, CommandKind::ShowRecents);
        assert_eq!(command.title, "Open Recently");
    }

    #[test]
    fn registry_titles_and_categories() {
        let entries: Vec<(&str, &str)> = super::COMMANDS
            .iter()
            .map(|c| (c.title, c.category))
            .collect();
        let mut expected: Vec<(&str, &str)> = vec![
            ("Save File", "File"),
            ("Format File", "File"),
            ("Open File", "File"),
            ("Open Recently", "File"),
            ("Open Folder", "File"),
            ("Switch To Configuration", "View"),
            ("Toggle Sidebar", "View"),
            ("Toggle Markdown Preview", "View"),
            ("Clear Recent Items", "Workspace"),
        ];
        expected.sort();
        let mut actual = entries.clone();
        actual.sort();
        assert_eq!(actual, expected);
        assert_eq!(entries.len(), 9);
    }

    #[test]
    fn markdown_preview_command_needs_a_markdown_document() {
        let command = super::by_id("toggle-markdown-preview").unwrap();
        let markdown = PaletteContext { active_is_markdown: true };
        let other = PaletteContext { active_is_markdown: false };
        assert!((command.available)(&markdown));
        assert!(!(command.available)(&other));
        // Every other command is context-independent.
        for command in super::COMMANDS.iter().filter(|c| c.id != "toggle-markdown-preview") {
            assert!((command.available)(&other), "{} hides without context", command.id);
        }
    }

    #[test]
    fn command_palette_item_carries_metadata() {
        let command: &Command = super::by_id("save-file").unwrap();
        let item = command.palette_item();
        assert_eq!(item.title, "Save File");
        assert_eq!(item.subtitle.as_deref(), Some("File"));
        assert!(matches!(&item.icon, CommandIcon::Phosphor(_)));
        assert!(matches!(&item.target, PaletteTarget::Registered(_)));
    }

    #[test]
    fn recent_list_builds_from_recent_items() {
        let mut config = crate::settings::Config::default();
        config.recent_items = vec![RecentItem {
            path: PathBuf::from("/tmp/rustfmt.toml"),
            is_dir: false,
        }];
        let items = recent_items(&config);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "rustfmt.toml");
        assert_eq!(items[0].subtitle.as_deref(), Some("/tmp/rustfmt.toml"));
        assert!(matches!(&items[0].icon, CommandIcon::File(_)));
    }

    #[test]
    fn recent_folders_get_folder_icon() {
        let mut config = crate::settings::Config::default();
        config.recent_items = vec![RecentItem {
            path: PathBuf::from("/tmp/project"),
            is_dir: true,
        }];
        let items = recent_items(&config);
        assert!(matches!(&items[0].icon, CommandIcon::Folder));
    }
}
