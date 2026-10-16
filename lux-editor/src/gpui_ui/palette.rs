//! The command palette: the application's actions, searchable and keyboard
//! driven, drawn over the editor.
//!
//! Edit-menu actions are deliberately absent: the palette's query input holds
//! the focus, so a dispatched `Undo` or `Paste` would land in the query box
//! instead of the editor.

use super::actions::{
    CloseTab, NewFile, OpenFile, OpenFolder, OpenSettings, Quit, Save, SaveAs, ToggleMarkdownPreview,
    ToggleSidebar,
};
use super::theme::Palette;
use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::*;

/// Sent when the palette wants to be hidden: Escape, or a confirmed command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteEvent {
    Dismissed,
}

pub struct CommandPalette {
    state: Entity<CommandState>,
    /// Focus the query input once, on the frame after the palette is shown.
    focus_pending: bool,
}

impl CommandPalette {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            state: cx.new(|cx| CommandState::new(window, cx)),
            focus_pending: true,
        }
    }

    /// Ask for the query input focus on the next frame, and select its text so
    /// typing replaces the previous query.
    pub fn request_focus(&mut self, cx: &mut Context<Self>) {
        self.focus_pending = true;
        cx.notify();
    }

    fn dismiss(this: &WeakEntity<Self>, cx: &mut App) {
        this.update(cx, |_, cx| cx.emit(PaletteEvent::Dismissed)).ok();
    }

    fn commands() -> [CommandGroup; 2] {
        [
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
            ]),
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
            ]),
        ]
    }
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
        let [files, view] = Self::commands();

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
                        Command::new(&self.state)
                            .bordered(false)
                            .max_h(px(384.0))
                            .placeholder("Type a command…")
                            .group(files)
                            .group(view)
                            .empty(|_, _, _| div().p_4().text_sm().child("No matching commands"))
                            .on_confirm({
                                let this = this.clone();
                                move |_, _, cx| Self::dismiss(&this, cx)
                            })
                            .on_cancel({
                                let this = this.clone();
                                move |_, cx| Self::dismiss(&this, cx)
                            }),
                    ),
            )
    }
}
