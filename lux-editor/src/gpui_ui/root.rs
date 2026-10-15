//! Root view: owns the application state, drains the background event channel
//! and renders the shell (title bar with the application menu, sidebar file
//! tree, tab strip, editor and status bar).

use super::actions::{
    CloseTab, NewFile, OpenCommandPalette, OpenFile, OpenFolder, Quit, Save, SaveAs,
    ToggleMarkdownPreview, ToggleSidebar,
};
use super::document::{OpenDocument, read_text};
use super::language::language_for;
use super::runtime::Runtime;
use super::state::{LuxState, TreeView};
use super::tabs::{Tab, Tabs};
use super::theme::{Palette, apply};
use crate::events::{CustomEvent, WorkspaceEvent};
use crate::settings::Config;
use crate::theme::ThemeChoice;
use crate::workspace::{Entry, FileTree, Workspace};
use gpui_kit::component::TitleBar;
use gpui_kit::component::h_flex;
use gpui_kit::component::input::{EditorState, InputEvent};
use gpui_kit::component::menu::AppMenuBar;
use gpui_kit::*;
use gpui_kit::prelude::FluentBuilder as _;
use std::path::{Path, PathBuf};

/// Housekeeping tick: recent-items flush debounce, session snapshot. The editor
/// owns its own caret blink, so this stays coarse.
const TICK: std::time::Duration = std::time::Duration::from_millis(500);

pub struct Root {
    state: LuxState,
    menu_bar: Entity<AppMenuBar>,
    sidebar_visible: bool,
    focus_handle: FocusHandle,
    /// Last title pushed to the OS, so render does not churn the window.
    last_title: Option<String>,
}

impl Root {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = Config::load();
        apply(
            cx,
            ThemeChoice::from_value(&config.settings.theme.choice),
            &config.settings.font.family,
            config.settings.font.size,
        );

        let (runtime, mut wake_rx) = Runtime::new();
        let menu_bar = AppMenuBar::new(cx);
        let mut root = Self {
            state: LuxState {
                runtime,
                config,
                workspace: Workspace::default(),
                tabs: Tabs::default(),
                tree: TreeView::default(),
            },
            menu_bar,
            sidebar_visible: true,
            focus_handle: cx.focus_handle(),
            last_title: None,
        };

        cx.spawn(async move |this, cx| {
            while wake_rx.recv().await.is_some() {
                let alive = this.update(cx, |root, cx| {
                    root.pump(cx);
                    cx.notify();
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                if this.update(cx, |root, _cx| root.tick()).is_err() {
                    break;
                }
            }
        })
        .detach();

        if let Some(path) = std::env::args().nth(1) {
            root.open_path(PathBuf::from(path), window, cx);
        }

        root.sync_window_title(window);
        root
    }

    /// Open a path from the CLI, the welcome page or the file tree: a directory
    /// becomes the workspace, a file is loaded into a tab.
    fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if path.is_dir() {
            self.open_folder(path, cx);
        } else {
            self.open_file(path, window, cx);
        }
    }

    fn open_folder(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let exclude = self.state.config.settings.explorer.exclude.clone();
        self.state.workspace.path = Some(path.clone());
        self.state.workspace.file_tree = Some(FileTree::new(&path, &exclude));
        self.state.workspace.watcher = Workspace::start_watcher(
            &path,
            &exclude,
            self.state.runtime.event_tx.clone(),
            self.state.runtime.wake.clone(),
        );
        self.state.tree.expanded.clear();
        self.state.tree.expanded.insert(path.clone());
        self.state.config.add_recent(path, true);
        cx.notify();
    }

    fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.state.tabs.index_of(&path) {
            self.state.tabs.active = index;
            self.sync_window_title(window);
            cx.notify();
            return;
        }
        let Some(text) = read_text(&path) else {
            log::warn!("refusing to open non-text file {}", path.display());
            return;
        };

        let language = language_for(&path);
        let editor = cx.new(|cx| {
            let mut editor = EditorState::new(window, cx).line_number(true);
            if let Some(language) = language {
                editor = editor.language(language);
            }
            editor.default_value(text)
        });

        self.push_editor(editor, Some(path.clone()), cx);
        self.state.config.add_recent(path, false);
        self.sync_window_title(window);
        cx.notify();
    }

    /// Create the window's first tab: an empty untitled document.
    fn new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let editor = cx.new(|cx| EditorState::new(window, cx).line_number(true));
        self.push_editor(editor, None, cx);
        self.sync_window_title(window);
        cx.notify();
    }

    /// Register an editor as a tab and subscribe to its change events.
    fn push_editor(
        &mut self,
        editor: Entity<EditorState>,
        path: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        let id = self.state.tabs.next_id();
        let changes = cx.subscribe(&editor, move |root, _editor, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                root.state.tabs.mark_dirty(id);
                cx.notify();
            }
        });
        self.state.tabs.push(Tab {
            id,
            doc: OpenDocument {
                editor,
                path,
                dirty: false,
                _changes: changes,
            },
        });
    }

    /// Save the active document, prompting for a path when it has none.
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.state.tabs.active_index() else {
            return;
        };
        self.save_index(index, window, cx);
    }

    /// Save tab `index`; returns whether the write landed. A tab without a path
    /// goes through the Save As dialog.
    fn save_index(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(path) = self.state.tabs.get(index).and_then(|doc| doc.path.clone()) else {
            return self.save_index_as(index, window, cx);
        };
        let Some(text) = self.state.tabs.get(index).map(|doc| doc.text(cx)) else {
            return false;
        };
        match crate::atomic::write(&path, text.as_bytes()) {
            Ok(()) => {
                if let Some(doc) = self.state.tabs.get_mut(index) {
                    doc.dirty = false;
                }
                self.state.config.add_recent(path, false);
                self.sync_window_title(window);
                cx.notify();
                true
            }
            Err(err) => {
                log::error!("failed to save {}: {err}", path.display());
                false
            }
        }
    }

    /// Save As: ask for a destination, then write there and adopt the path.
    fn save_index_as(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(doc) = self.state.tabs.get(index) else {
            return false;
        };
        let text = doc.text(cx);
        let current = doc.path.clone();
        let mut dialog = rfd::FileDialog::new();
        if let Some(dir) = current.as_ref().and_then(|path| path.parent()) {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.save_file() else {
            return false;
        };
        match crate::atomic::write(&path, text.as_bytes()) {
            Ok(()) => {
                if let Some(doc) = self.state.tabs.get_mut(index) {
                    doc.set_path(path.clone());
                }
                self.state.config.add_recent(path, false);
                self.sync_window_title(window);
                cx.notify();
                true
            }
            Err(err) => {
                log::error!("failed to save {}: {err}", path.display());
                false
            }
        }
    }

    fn save_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.state.tabs.active_index() {
            self.save_index_as(index, window, cx);
        }
    }

    fn open_file_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.open_file(path, window, cx);
        }
    }

    fn open_folder_dialog(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.open_folder(path, cx);
        }
    }

    /// Close the active tab, prompting first when it has unsaved changes.
    fn close_active_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.state.tabs.active_index() {
            self.close_tab(index, window, cx);
        }
    }

    fn close_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.state.tabs.is_dirty(index) {
            let name = self
                .state
                .tabs
                .get(index)
                .map(|doc| doc.title())
                .unwrap_or_default();
            let answer = rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description(format!("Save changes to {name} before closing?"))
                .set_buttons(rfd::MessageButtons::YesNoCancel)
                .show();
            match answer {
                rfd::MessageDialogResult::Yes => {
                    if !self.save_index(index, window, cx) {
                        return;
                    }
                }
                rfd::MessageDialogResult::No => {}
                _ => return,
            }
        }
        self.state.tabs.close(index);
        self.sync_window_title(window);
        cx.notify();
    }

    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_visible = !self.sidebar_visible;
        cx.notify();
    }

    fn toggle_dir(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.state.tree.expanded.remove(&path) {
            self.state.tree.expanded.insert(path);
        }
        cx.notify();
    }

    /// Background events: a workspace file change rescans the loaded tree; a
    /// configuration change reloads the settings.
    fn pump(&mut self, cx: &mut Context<Self>) {
        while let Ok(event) = self.state.runtime.event_rx.try_recv() {
            match event {
                CustomEvent::Workspace(WorkspaceEvent::FileChange) => {
                    if let Some(tree) = self.state.workspace.file_tree.as_mut() {
                        tree.refresh();
                    }
                    cx.notify();
                }
                CustomEvent::App(crate::events::AppEvent::ConfigChange) => {
                    if self.state.config.reload_settings() {
                        let settings = &self.state.config.settings;
                        apply(
                            cx,
                            ThemeChoice::from_value(&settings.theme.choice),
                            &settings.font.family,
                            settings.font.size,
                        );
                        cx.notify();
                    }
                }
                _ => {}
            }
        }
    }

    fn tick(&mut self) {
        self.state.config.flush_recent();
    }

    /// Reflect the active document (name + dirty marker) in the OS window title.
    fn sync_window_title(&mut self, window: &mut Window) {
        let title = match self.state.tabs.active_doc() {
            Some(doc) => {
                let marker = if doc.dirty { " •" } else { "" };
                format!("{}{marker} — Lux", doc.title())
            }
            None => "Lux".to_string(),
        };
        if self.last_title.as_deref() != Some(title.as_str()) {
            window.set_window_title(&title);
            self.last_title = Some(title);
        }
    }

    fn sidebar_rows(&mut self) -> Vec<Row> {
        let mut rows = Vec::new();
        if let Some(tree) = self.state.workspace.file_tree.as_mut() {
            collect_rows(tree, &self.state.tree.expanded, &mut rows);
        }
        rows
    }
}

struct Row {
    path: PathBuf,
    name: String,
    depth: usize,
    is_dir: bool,
    ignored: bool,
}

fn collect_rows(tree: &mut FileTree, expanded: &std::collections::HashSet<PathBuf>, out: &mut Vec<Row>) {
    let root = tree.root().to_path_buf();
    collect_dir(tree, expanded, &root, 0, out);
}

fn collect_dir(
    tree: &mut FileTree,
    expanded: &std::collections::HashSet<PathBuf>,
    dir: &Path,
    depth: usize,
    out: &mut Vec<Row>,
) {
    for entry in tree.children(dir).iter() {
        let (path, is_dir, ignored) = match entry {
            Entry::File { path, ignored } => (path.clone(), false, *ignored),
            Entry::Directory { path, ignored } => (path.clone(), true, *ignored),
        };
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if is_dir && expanded.contains(&path) {
            collect_dir(tree, expanded, &path, depth + 1, out);
        }
        out.push(Row {
            path,
            name,
            depth,
            is_dir,
            ignored,
        });
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::get(cx);
        self.sync_window_title(window);

        let title_bar = TitleBar::new().child(
            h_flex()
                .flex_1()
                .items_center()
                .gap_2()
                .overflow_hidden()
                .child(self.menu_bar.clone())
                .child(
                    div()
                        .text_xs()
                        .text_color(palette.weak)
                        .truncate()
                        .child(self.window_caption()),
                ),
        );

        let mut body = div().flex().flex_row().flex_1().min_h_0();
        if self.sidebar_visible {
            body = body.child(self.render_sidebar(&palette, cx));
        }
        body = body.child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .h_full()
                .child(self.render_tabs(&palette, cx))
                .child(self.render_editor_area(&palette)),
        );

        div()
            .id("lux-root")
            .track_focus(&self.focus_handle)
            .key_context("Lux")
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.window_bg)
            .text_color(palette.text)
            .on_action(cx.listener(|root, _: &NewFile, window, cx| root.new_file(window, cx)))
            .on_action(cx.listener(|root, _: &OpenFile, window, cx| {
                root.open_file_dialog(window, cx)
            }))
            .on_action(cx.listener(|root, _: &OpenFolder, _window, cx| {
                root.open_folder_dialog(cx)
            }))
            .on_action(cx.listener(|root, _: &Save, window, cx| root.save(window, cx)))
            .on_action(cx.listener(|root, _: &SaveAs, window, cx| root.save_as(window, cx)))
            .on_action(cx.listener(|root, _: &CloseTab, window, cx| {
                root.close_active_tab(window, cx)
            }))
            .on_action(cx.listener(|root, _: &ToggleSidebar, _window, cx| {
                root.toggle_sidebar(cx)
            }))
            .on_action(cx.listener(|_root, _: &OpenCommandPalette, _window, _cx| {
                // Implemented with the command palette view.
            }))
            .on_action(cx.listener(|_root, _: &ToggleMarkdownPreview, _window, _cx| {
                // Implemented with the markdown preview view.
            }))
            .on_action(cx.listener(|_root, _: &Quit, _window, cx| cx.quit()))
            .child(title_bar)
            .child(body)
            .child(self.render_status_bar(&palette, cx))
    }
}

impl Root {
    /// The active document name shown next to the menu in the title bar.
    fn window_caption(&self) -> String {
        match self.state.tabs.active_doc() {
            Some(doc) => doc
                .path
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| doc.title()),
            None => "Lux".to_string(),
        }
    }

    fn render_sidebar(&mut self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let mut sidebar = div()
            .flex()
            .flex_col()
            .w(px(240.0))
            .h_full()
            .flex_shrink_0()
            .bg(palette.panel_bg)
            .border_r_1()
            .border_color(palette.border)
            .overflow_hidden()
            .py_1();
        for (row_index, row) in self.sidebar_rows().into_iter().enumerate() {
            let Row {
                path,
                name,
                depth,
                is_dir,
                ignored,
            } = row;
            let color = if ignored { palette.weak } else { palette.text };
            let label = if is_dir {
                format!(
                    "{} {name}",
                    if self.state.tree.expanded.contains(&path) {
                        "▾"
                    } else {
                        "▸"
                    }
                )
            } else {
                name
            };
            sidebar = sidebar.child(
                div()
                    .id(("sidebar-row", row_index))
                    .flex()
                    .items_center()
                    .h(px(22.0))
                    .flex_shrink_0()
                    .pl(px(8.0 + depth as f32 * 12.0))
                    .pr_2()
                    .text_sm()
                    .text_color(color)
                    .cursor_pointer()
                    .hover(|style| style.bg(palette.hover))
                    .on_click(cx.listener(move |root, _event, window, cx| {
                        if is_dir {
                            root.toggle_dir(path.clone(), cx);
                        } else {
                            root.open_file(path.clone(), window, cx);
                        }
                    }))
                    .child(label),
            );
        }
        sidebar
    }

    fn render_tabs(&mut self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let mut strip = div()
            .flex()
            .flex_row()
            .h(px(32.0))
            .flex_shrink_0()
            .bg(palette.panel_bg)
            .border_b_1()
            .border_color(palette.border);
        for (index, tab) in self.state.tabs.tabs.iter().enumerate() {
            let mut label = tab.doc.title();
            if tab.doc.dirty {
                label.push_str(" •");
            }
            let active = index == self.state.tabs.active;
            strip = strip.child(
                div()
                    .id(("tab", index))
                    .flex()
                    .items_center()
                    .gap_2()
                    .h_full()
                    .px_3()
                    .text_sm()
                    .cursor_pointer()
                    .border_r_1()
                    .border_color(palette.border)
                    .when(active, |item| item.bg(palette.window_bg).text_color(palette.text))
                    .when(!active, |item| item.text_color(palette.weak))
                    .on_click(cx.listener(move |root, _event, window, cx| {
                        root.state.tabs.active = index;
                        root.sync_window_title(window);
                        cx.notify();
                    }))
                    .child(label)
                    .child(
                        div()
                            .id(("tab-close", index))
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(16.0))
                            .h(px(16.0))
                            .rounded_sm()
                            .text_xs()
                            .hover(|style| style.bg(palette.active))
                            .on_click(cx.listener(move |root, _event, window, cx| {
                                cx.stop_propagation();
                                root.close_tab(index, window, cx);
                            }))
                            .child("×"),
                    ),
            );
        }
        strip
    }

    fn render_editor_area(&mut self, palette: &Palette) -> AnyElement {
        if let Some(doc) = self.state.tabs.active_doc() {
            div()
                .flex_1()
                .min_h_0()
                .child(doc.view().size_full())
                .into_any_element()
        } else {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(palette.weak)
                .child("No open files")
                .into_any_element()
        }
    }

    fn render_status_bar(&mut self, palette: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let (caret, language, dirty) = match self.state.tabs.active_doc() {
            Some(doc) => {
                let (line, column) = doc.caret_line_column(cx);
                let language = doc
                    .path
                    .as_deref()
                    .and_then(language_for)
                    .unwrap_or("Plain Text")
                    .to_string();
                (format!("Ln {line}, Col {column}"), language, doc.dirty)
            }
            None => ("Ln 1, Col 1".to_string(), "Plain Text".to_string(), false),
        };
        let workspace = self
            .state
            .workspace
            .path
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();

        h_flex()
            .flex_shrink_0()
            .h(px(24.0))
            .px_3()
            .gap_3()
            .text_xs()
            .bg(palette.panel_bg)
            .text_color(palette.weak)
            .border_t_1()
            .border_color(palette.border)
            .child(caret)
            .child(language)
            .when(dirty, |bar| bar.child("Modified"))
            .child(div().flex_1())
            .child(workspace)
    }
}
