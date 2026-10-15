//! Root view: owns the application state, drains the background event channel
//! and renders the shell (sidebar file tree, tab strip, editor).

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
use gpui_kit::component::input::{EditorState, InputEvent};
use gpui_kit::*;
use gpui_kit::{InteractiveElement as _, StatefulInteractiveElement as _};
use std::path::{Path, PathBuf};

/// Housekeeping tick: recent-items flush debounce, session snapshot. The editor
/// owns its own caret blink, so this stays coarse.
const TICK: std::time::Duration = std::time::Duration::from_millis(500);

pub struct Root {
    state: LuxState,
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
        let mut root = Self {
            state: LuxState {
                runtime,
                config,
                workspace: Workspace::default(),
                tabs: Tabs::default(),
                tree: TreeView::default(),
            },
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
                path: Some(path.clone()),
                dirty: false,
                _changes: changes,
            },
        });
        self.state.config.add_recent(path, false);
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::get(cx);

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
                format!("{} {name}", if self.state.tree.expanded.contains(&path) { "▾" } else { "▸" })
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
            let mut item = div()
                .id(("tab", index))
                .flex()
                .items_center()
                .h_full()
                .px_3()
                .text_sm()
                .cursor_pointer()
                .border_r_1()
                .border_color(palette.border)
                .on_click(cx.listener(move |root, _event, _window, cx| {
                    root.state.tabs.active = index;
                    cx.notify();
                }))
                .child(label);
            if index == self.state.tabs.active {
                item = item.bg(palette.window_bg).text_color(palette.text);
            } else {
                item = item.text_color(palette.weak);
            }
            strip = strip.child(item);
        }

        let editor_area = if let Some(doc) = self.state.tabs.active_doc() {
            div()
                .flex_1()
                .h_full()
                .child(doc.view().size_full())
                .into_any_element()
        } else {
            div()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(palette.weak)
                .child("No open files")
                .into_any_element()
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.window_bg)
            .text_color(palette.text)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(sidebar)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .child(strip)
                            .child(editor_area),
                    ),
            )
    }
}
