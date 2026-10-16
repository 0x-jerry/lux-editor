//! The command palette: the application's actions, searchable and keyboard
//! driven, drawn over the editor.
//!
//! Edit-menu actions are deliberately absent: the palette's query input holds
//! the focus, so a dispatched `Undo` or `Paste` would land in the query box
//! instead of the editor.

use super::actions::{
    ClearRecentItems, CloseTab, NewFile, OpenFile, OpenFolder, OpenSettings, Quit, Save, SaveAs,
    ToggleMarkdownPreview, ToggleSidebar,
};
use super::theme::Palette;
use crate::settings::types::RecentItem;
use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::component::IndexPath;
use gpui_kit::*;
use std::path::PathBuf;

/// What the palette asks its host to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteEvent {
    /// Hide the palette: Escape, a dispatched command, or a click outside it.
    Dismissed,
    /// Open a recent item, which has no action of its own to dispatch.
    Open(PathBuf),
}

pub struct CommandPalette {
    state: Entity<CommandState>,
    /// The recent files and folders, refreshed by the root each time the palette
    /// is shown.
    recents: Vec<RecentItem>,
    /// Focus the query input once, on the frame after the palette is shown.
    focus_pending: bool,
}

impl CommandPalette {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            state: cx.new(|cx| CommandState::new(window, cx)),
            recents: Vec::new(),
            focus_pending: true,
        }
    }

    /// Ask for the query input focus on the next frame, and select its text so
    /// typing replaces the previous query.
    pub fn request_focus(&mut self, cx: &mut Context<Self>) {
        self.focus_pending = true;
        cx.notify();
    }

    pub fn set_recents(&mut self, recents: Vec<RecentItem>, cx: &mut Context<Self>) {
        self.recents = recents;
        cx.notify();
    }

    fn dismiss(this: &WeakEntity<Self>, cx: &mut App) {
        this.update(cx, |_, cx| cx.emit(PaletteEvent::Dismissed)).ok();
    }

    /// The groups to show, and the section holding the recent items — the only
    /// rows without an action of their own, so the confirm callback has to
    /// resolve them by position.
    fn groups(&self) -> (Vec<CommandGroup>, Option<usize>) {
        let mut groups = Vec::new();
        if !self.recents.is_empty() {
            groups.push(Self::recent_group(&self.recents));
        }
        groups.push(Self::file_group());
        groups.push(Self::view_group());
        let recent_section = (!self.recents.is_empty()).then_some(0);
        (groups, recent_section)
    }

    /// The recent items, with the command that empties the list.
    fn recent_group(recents: &[RecentItem]) -> CommandGroup {
        let rows = recents.iter().map(|item| {
            let mut keywords = vec![item.path.display().to_string()];
            if item.is_dir {
                keywords.push("folder".to_string());
            }
            CommandItem::new()
                .label(file_name(&item.path))
                .keywords(keywords)
        });
        CommandGroup::new().label("Recent").items(
            rows.chain([CommandItem::new()
                .label("Clear Recent Items")
                .keywords(["forget", "history"])
                .action(Box::new(ClearRecentItems))]),
        )
    }

    fn file_group() -> CommandGroup {
        CommandGroup::new().label("File").items([
            CommandItem::new()
                .label("New File")
                .keywords(["create", "untitled"])
                .action(Box::new(NewFile)),
            CommandItem::new()
                .label("Open File…")
                .keywords(["open", "load"])
                .action(Box::new(OpenFile)),
            CommandItem::new()
                .label("Open Folder…")
                .keywords(["workspace", "project"])
                .action(Box::new(OpenFolder)),
            CommandItem::new()
                .label("Save")
                .keywords(["write"])
                .action(Box::new(Save)),
            CommandItem::new()
                .label("Save As…")
                .keywords(["write", "rename"])
                .action(Box::new(SaveAs)),
            CommandItem::new()
                .label("Close Tab")
                .keywords(["close"])
                .action(Box::new(CloseTab)),
            CommandItem::new()
                .label("Quit Lux")
                .keywords(["exit", "close window"])
                .action(Box::new(Quit)),
        ])
    }

    fn view_group() -> CommandGroup {
        CommandGroup::new().label("View").items([
            CommandItem::new()
                .label("Toggle Sidebar")
                .keywords(["explorer", "files"])
                .action(Box::new(ToggleSidebar)),
            CommandItem::new()
                .label("Toggle Markdown Preview")
                .keywords(["preview", "render"])
                .action(Box::new(ToggleMarkdownPreview)),
            CommandItem::new()
                .label("Settings")
                .keywords(["preferences", "config", "options"])
                .action(Box::new(OpenSettings)),
        ])
    }
}

/// A recent item's row label: its file name, or the full path when it has none.
fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

impl EventEmitter<PaletteEvent> for CommandPalette {}

impl Render for CommandPalette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::get(cx);
        if self.focus_pending {
            self.focus_pending = false;
            let state = self.state.clone();
            window.on_next_frame(move |window, cx| {
                state.update(cx, |state, cx| state.focus(window, cx));
            });
        }

        let this = cx.weak_entity();
        let (groups, recent_section) = self.groups();
        let recents = self.recents.clone();

        let mut command = Command::new(&self.state)
            .bordered(false)
            .max_h(px(384.0))
            .placeholder("Type a command…");
        for group in groups {
            command = command.group(group);
        }

        div()
            .absolute()
            .inset_0()
            .flex()
            .justify_center()
            .items_start()
            .pt(px(88.0))
            .bg(hsla(0.0, 0.0, 0.0, 0.35))
            .on_mouse_down(MouseButton::Left, {
                let this = this.clone();
                move |_, _, cx| Self::dismiss(&this, cx)
            })
            .child(
                div()
                    .w(px(560.0))
                    .rounded_lg()
                    .border_1()
                    .border_color(palette.border)
                    .bg(palette.raised)
                    .overflow_hidden()
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        command
                            .empty(|_, _, _| div().p_4().text_sm().child("No matching commands"))
                            .on_confirm({
                                let this = this.clone();
                                move |index_path: IndexPath, _, cx| {
                                    if Some(index_path.section) == recent_section
                                        && let Some(item) = recents.get(index_path.row)
                                    {
                                        let path = item.path.clone();
                                        this.update(cx, |_, cx| cx.emit(PaletteEvent::Open(path)))
                                            .ok();
                                    }
                                    Self::dismiss(&this, cx);
                                }
                            })
                            .on_cancel({
                                let this = this.clone();
                                move |_, cx| Self::dismiss(&this, cx)
                            }),
                    ),
            )
    }
}
