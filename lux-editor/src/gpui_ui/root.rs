//! Root view: owns the application state, drains the background event channel
//! and renders the shell (title bar with the application menu, sidebar file
//! tree, tab strip, editor and status bar).

use super::actions::{
    CloseTab, NewFile, OpenCommandPalette, OpenFile, OpenFolder, OpenSettings, Quit, Save, SaveAs,
    ToggleMarkdownPreview, ToggleSidebar,
};
use super::document::OpenDocument;
use super::language::language_for;
use super::markdown;
use super::palette::{CommandPalette, PaletteEvent};
use super::runtime::Runtime;
use super::settings_view::{SettingsEvent, SettingsView};
use super::state::{LuxState, TreeView};
use super::tabs::{Tab, TabContent, Tabs};
use super::theme::{Palette, apply};
use super::views::{self, Content, Notice};
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

/// The answer to "this tab has unsaved changes".
enum Decision {
    Save,
    Discard,
    Cancel,
}

/// Whether an OS appearance is one of the light ones.
fn is_light(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Light | WindowAppearance::VibrantLight
    )
}

pub struct Root {
    state: LuxState,
    menu_bar: Entity<AppMenuBar>,
    /// The command palette, rendered as an overlay while `palette_open`.
    palette: Entity<CommandPalette>,
    _palette_events: Subscription,
    /// The settings page; the tab that shows it holds no state of its own.
    settings_view: Entity<SettingsView>,
    _settings_events: Subscription,
    palette_open: bool,
    sidebar_visible: bool,
    /// The settings tab changed the configuration; the write is pending.
    settings_dirty: bool,
    /// The OS appearance, which the `auto` theme choice follows.
    system_is_light: bool,
    focus_handle: FocusHandle,
    /// Last title pushed to the OS, so render does not churn the window.
    last_title: Option<String>,
}

impl Root {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = Config::load();
        let system_is_light = is_light(window.appearance());
        apply(
            cx,
            ThemeChoice::from_value(&config.settings.theme.choice),
            &config.settings.font.family,
            config.settings.font.size,
            system_is_light,
        );

        let (runtime, mut wake_rx) = Runtime::new();
        let menu_bar = AppMenuBar::new(cx);
        let palette = cx.new(|cx| CommandPalette::new(window, cx));
        let settings_view = cx.new(|cx| SettingsView::new(&config.settings, window, cx));
        let _palette_events = cx.subscribe(&palette, |root, _palette, event: &PaletteEvent, cx| {
            if matches!(event, PaletteEvent::Dismissed) {
                root.palette_open = false;
                cx.notify();
            }
        });
        let _settings_events = cx.subscribe(&settings_view, Self::on_settings_event);

        let mut root = Self {
            state: LuxState {
                runtime,
                config,
                workspace: Workspace::default(),
                tabs: Tabs::default(),
                tree: TreeView::default(),
            },
            menu_bar,
            palette,
            _palette_events,
            settings_view,
            _settings_events,
            palette_open: false,
            sidebar_visible: true,
            settings_dirty: false,
            system_is_light,
            focus_handle: cx.focus_handle(),
            last_title: None,
        };

        // The window's own close button goes through the same gate as Quit. The
        // platform keeps a single should-close callback, so this registration is
        // the window's only one.
        window.on_window_should_close(cx, {
            let this = cx.weak_entity();
            move |window, cx| {
                this.update(cx, |root, cx| root.confirm_all_dirty(window, cx))
                    .unwrap_or(true)
            }
        });

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
    /// Open whatever the command line named: a folder becomes the workspace, a
    /// file is loaded as a tab. Anything else is ignored.
    fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.canonicalize().unwrap_or(path);
        if path.is_dir() {
            self.open_folder(path, cx);
        } else if path.is_file() {
            self.open_file(path, window, cx);
        } else {
            log::warn!("ignoring command-line path {}", path.display());
        }
    }

    fn open_folder(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let path = path.canonicalize().unwrap_or(path);
        self.state.workspace.path = Some(path.clone());
        self.rescan_workspace();
        self.state.tree.expanded.clear();
        self.state.tree.expanded.insert(path.clone());
        self.state.config.add_recent(path, true);
        cx.notify();
    }

    /// Rescan the workspace root and restart its watcher, which is how a change
    /// to the exclusion rules takes effect.
    fn rescan_workspace(&mut self) {
        let Some(path) = self.state.workspace.path.clone() else {
            return;
        };
        let exclude = self.state.config.settings.explorer.exclude.clone();
        self.state.workspace.file_tree = Some(FileTree::new(&path, &exclude));
        self.state.workspace.watcher = Workspace::start_watcher(
            &path,
            &exclude,
            self.state.runtime.event_tx.clone(),
            self.state.runtime.wake.clone(),
        );
    }

    fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.state.tabs.index_of(&path) {
            self.activate(index, window, cx);
            return;
        }

        let (content, list_as_recent) = match views::classify(&path) {
            Content::Text(text) => (
                self.new_text_tab(text, Some(path.clone()), window, cx),
                true,
            ),
            Content::Image => (TabContent::Image(path.clone()), true),
            Content::Binary => (
                TabContent::Notice {
                    path: path.clone(),
                    kind: Notice::Binary,
                },
                false,
            ),
            Content::Missing => (
                TabContent::Notice {
                    path: path.clone(),
                    kind: Notice::Missing,
                },
                false,
            ),
        };

        self.push_content(content);
        if list_as_recent {
            self.state.config.add_recent(path, false);
        }
        self.sync_window_title(window);
        cx.notify();
    }

    /// Create the window's first tab: an empty untitled document.
    fn new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let content = self.new_text_tab(String::new(), None, window, cx);
        self.push_content(content);
        self.sync_window_title(window);
        cx.notify();
    }

    /// Load `text` into a new editor and wrap it as a text tab body, wiring the
    /// editor's change events to the tab's dirty flag.
    fn new_text_tab(
        &mut self,
        text: String,
        path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> TabContent {
        let language = path.as_deref().and_then(language_for);
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
        TabContent::Text(OpenDocument {
            editor,
            path,
            dirty: false,
            preview: false,
            _changes: changes,
        })
    }

    /// Add a tab and make it the active one.
    fn push_content(&mut self, content: TabContent) {
        let id = self.state.tabs.next_id();
        self.state.tabs.push(Tab { id, content });
    }

    /// Show the settings tab, creating it the first time it is asked for.
    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.state.tabs.settings_index() {
            self.activate(index, window, cx);
            return;
        }
        let settings = self.state.config.settings.clone();
        self.settings_view
            .update(cx, |view, cx| view.sync(&settings, cx));
        self.push_content(TabContent::Settings);
        self.sync_window_title(window);
        cx.notify();
    }

    fn activate(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.state.tabs.active = index;
        self.sync_window_title(window);
        cx.notify();
    }

    /// Flip the active document between its source and its rendered markdown.
    fn toggle_preview(&mut self, cx: &mut Context<Self>) {
        let Some(doc) = self.state.tabs.active_doc_mut() else {
            return;
        };
        let Some(path) = doc.path.clone() else {
            return;
        };
        if !markdown::is_markdown(&path) {
            log::debug!("no preview for {}", path.display());
            return;
        }
        doc.preview = !doc.preview;
        cx.notify();
    }

    /// Show the command palette with the query box focused.
    fn open_palette(&mut self, cx: &mut Context<Self>) {
        self.palette_open = true;
        self.palette.update(cx, |palette, cx| palette.request_focus(cx));
        cx.notify();
    }

    /// Hide the palette. Every action it can dispatch does this first, so a
    /// dispatched command never leaves the overlay behind.
    fn dismiss_palette(&mut self, cx: &mut Context<Self>) {
        if self.palette_open {
            self.palette_open = false;
            cx.notify();
        }
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
        if !self.confirm_tab(index, window, cx) {
            return;
        }
        self.state.tabs.close(index);
        self.sync_window_title(window);
        cx.notify();
    }

    /// Ask about tab `index` when it has unsaved changes, and for a `Save`
    /// answer write it. Returns whether the tab may now be discarded.
    fn confirm_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.state.tabs.is_dirty(index) {
            return true;
        }
        match self.ask_about(index) {
            Decision::Save => self.save_index(index, window, cx),
            Decision::Discard => true,
            Decision::Cancel => false,
        }
    }

    /// Walk the unsaved documents, asking about each. Returns whether every one
    /// of them may be discarded, which is what a close or a quit needs.
    fn confirm_all_dirty(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        for index in 0..self.state.tabs.len() {
            if !self.confirm_tab(index, window, cx) {
                return false;
            }
        }
        true
    }

    /// Put the unsaved-changes question to the user for tab `index`.
    fn ask_about(&self, index: usize) -> Decision {
        let name = self
            .state
            .tabs
            .tab(index)
            .map(Tab::title)
            .unwrap_or_default();
        let answer = rfd::MessageDialog::new()
            .set_title("Unsaved changes")
            .set_description(format!("Save changes to {name}?"))
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show();
        match answer {
            rfd::MessageDialogResult::Yes => Decision::Save,
            rfd::MessageDialogResult::No => Decision::Discard,
            _ => Decision::Cancel,
        }
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
                        let settings = self.state.config.settings.clone();
                        self.refresh_theme(cx);
                        self.settings_view
                            .update(cx, |view, cx| view.sync(&settings, cx));
                        cx.notify();
                    }
                }
            }
        }
    }

    fn tick(&mut self) {
        self.state.config.flush_recent();
        self.save_settings();
    }

    /// Write settings edited in the settings tab, at most once per tick so a
    /// burst of keystrokes is one file write.
    fn save_settings(&mut self) {
        if !self.settings_dirty {
            return;
        }
        match Config::save_settings(&self.state.config.settings) {
            Ok(_) => self.settings_dirty = false,
            Err(err) => log::error!("failed to save settings: {err}"),
        }
    }

    /// Reflect the active tab (name + dirty marker) in the OS window title.
    fn sync_window_title(&mut self, window: &mut Window) {
        let title = match self.state.tabs.active_tab() {
            Some(tab) => {
                let marker = if tab.dirty() { " •" } else { "" };
                format!("{}{marker} — Lux", tab.title())
            }
            None => "Lux".to_string(),
        };
        if self.last_title.as_deref() != Some(title.as_str()) {
            window.set_window_title(&title);
            self.last_title = Some(title);
        }
    }

    /// Apply an edit from the settings tab: re-render whatever it affects and
    /// leave the write to the next tick, so typing is not one write per key.
    fn on_settings_event(
        &mut self,
        _view: Entity<SettingsView>,
        event: &SettingsEvent,
        cx: &mut Context<Self>,
    ) {
        let SettingsEvent::Changed(settings) = event;
        let settings = (**settings).clone();
        let current = &self.state.config.settings;
        let appearance_changed = settings.theme != current.theme || settings.font != current.font;
        let excludes_changed = settings.explorer.exclude != current.explorer.exclude;

        self.state.config.settings = settings;
        self.settings_dirty = true;
        if appearance_changed {
            self.refresh_theme(cx);
        }
        if excludes_changed {
            self.rescan_workspace();
        }
        cx.notify();
    }

    /// Re-publish the palette and editor font from the current settings.
    fn refresh_theme(&self, cx: &mut Context<Self>) {
        let settings = &self.state.config.settings;
        apply(
            cx,
            ThemeChoice::from_value(&settings.theme.choice),
            &settings.font.family,
            settings.font.size,
            self.system_is_light,
        );
    }

    /// Follow a change of the OS appearance while the theme choice is `auto`.
    fn sync_system_appearance(&mut self, window: &Window, cx: &mut Context<Self>) {
        let system_is_light = is_light(window.appearance());
        if system_is_light == self.system_is_light {
            return;
        }
        self.system_is_light = system_is_light;
        if ThemeChoice::from_value(&self.state.config.settings.theme.choice) == ThemeChoice::Auto {
            self.refresh_theme(cx);
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
        self.sync_system_appearance(window, cx);

        let title_bar = TitleBar::new()
            .on_close_window(cx.listener(|root, _: &ClickEvent, window, cx| {
                // The title bar's close button calls `remove_window`, which does
                // not consult the platform should-close callback, so it needs
                // the same gate as Quit.
                root.dismiss_palette(cx);
                if root.confirm_all_dirty(window, cx) {
                    window.remove_window();
                }
            }))
            .child(
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
                .child(self.render_editor_area(&palette, cx)),
        );

        let palette_open = self.palette_open;
        let palette_view = self.palette.clone();

        div()
            .id("lux-root")
            .track_focus(&self.focus_handle)
            .key_context("Lux")
            .size_full()
            .flex()
            .flex_col()
            .bg(palette.window_bg)
            .text_color(palette.text)
            .on_action(cx.listener(|root, _: &NewFile, window, cx| {
                root.dismiss_palette(cx);
                root.new_file(window, cx)
            }))
            .on_action(cx.listener(|root, _: &OpenFile, window, cx| {
                root.dismiss_palette(cx);
                root.open_file_dialog(window, cx)
            }))
            .on_action(cx.listener(|root, _: &OpenFolder, _window, cx| {
                root.dismiss_palette(cx);
                root.open_folder_dialog(cx)
            }))
            .on_action(cx.listener(|root, _: &Save, window, cx| {
                root.dismiss_palette(cx);
                root.save(window, cx)
            }))
            .on_action(cx.listener(|root, _: &SaveAs, window, cx| {
                root.dismiss_palette(cx);
                root.save_as(window, cx)
            }))
            .on_action(cx.listener(|root, _: &CloseTab, window, cx| {
                root.dismiss_palette(cx);
                root.close_active_tab(window, cx)
            }))
            .on_action(cx.listener(|root, _: &ToggleSidebar, _window, cx| {
                root.dismiss_palette(cx);
                root.toggle_sidebar(cx)
            }))
            .on_action(cx.listener(|root, _: &OpenCommandPalette, _window, cx| {
                root.open_palette(cx)
            }))
            .on_action(cx.listener(|root, _: &ToggleMarkdownPreview, _window, cx| {
                root.dismiss_palette(cx);
                root.toggle_preview(cx)
            }))
            .on_action(cx.listener(|root, _: &OpenSettings, window, cx| {
                root.dismiss_palette(cx);
                root.open_settings(window, cx)
            }))
            .on_action(cx.listener(|root, _: &Quit, window, cx| {
                root.dismiss_palette(cx);
                if root.confirm_all_dirty(window, cx) {
                    cx.quit();
                }
            }))
            .child(title_bar)
            .child(body)
            .child(self.render_status_bar(&palette, cx))
            .when(palette_open, move |root| root.child(palette_view))
    }
}

impl Root {
    /// The active tab name shown next to the menu in the title bar.
    fn window_caption(&self) -> String {
        match self.state.tabs.active_tab() {
            Some(tab) => tab
                .path()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| tab.title()),
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

    fn render_tabs(&self, palette: &Palette, cx: &Context<Self>) -> impl IntoElement {
        let mut strip = div()
            .flex()
            .flex_row()
            .h(px(32.0))
            .flex_shrink_0()
            .bg(palette.panel_bg)
            .border_b_1()
            .border_color(palette.border);
        for (index, tab) in self.state.tabs.tabs.iter().enumerate() {
            let mut label = tab.title();
            if tab.dirty() {
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
                        root.activate(index, window, cx)
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

    /// The body under the tab strip: whichever view the active tab holds.
    fn render_editor_area(&self, palette: &Palette, cx: &App) -> AnyElement {
        let Some(tab) = self.state.tabs.active_tab() else {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(palette.weak)
                .child("No open files")
                .into_any_element();
        };

        match &tab.content {
            TabContent::Text(doc) => {
                let path = doc.path.clone();
                if doc.preview && path.as_deref().is_some_and(markdown::is_markdown) {
                    return markdown::preview(tab.id, &doc.text(cx));
                }
                div()
                    .flex_1()
                    .min_h_0()
                    .child(doc.view().size_full())
                    .into_any_element()
            }
            TabContent::Image(path) => views::image(path),
            TabContent::Notice { path, kind } => views::notice(path, *kind, cx),
            TabContent::Settings => div()
                .flex_1()
                .min_h_0()
                .child(self.settings_view.clone())
                .into_any_element(),
        }
    }

    fn render_status_bar(&self, palette: &Palette, cx: &App) -> impl IntoElement {
        let (caret, language, dirty, preview) = match self.state.tabs.active_tab() {
            Some(tab) => match &tab.content {
                TabContent::Text(doc) => {
                    let (line, column) = doc.caret_line_column(cx);
                    let language = doc
                        .path
                        .as_deref()
                        .and_then(language_for)
                        .unwrap_or("Plain Text")
                        .to_string();
                    (
                        format!("Ln {line}, Col {column}"),
                        language,
                        doc.dirty,
                        doc.preview,
                    )
                }
                TabContent::Image(_) => (String::new(), "Image".to_string(), false, false),
                TabContent::Notice { kind, .. } => (
                    String::new(),
                    match kind {
                        Notice::Binary => "Binary",
                        Notice::Missing => "Unavailable",
                    }
                    .to_string(),
                    false,
                    false,
                ),
                TabContent::Settings => (String::new(), "Lux Settings".to_string(), false, false),
            },
            None => ("Ln 1, Col 1".to_string(), "Plain Text".to_string(), false, false),
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
            .when(preview, |bar| bar.child("Preview"))
            .when(dirty, |bar| bar.child("Modified"))
            .child(div().flex_1())
            .child(workspace)
    }
}
