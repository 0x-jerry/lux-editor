use crate::chrome::ui::widgets::file_type_icon;
use crate::component::Component;
use crate::events::{AppEvent, CustomEvent, WorkspaceEvent};
use crate::workspace::{Entry, FileTree};
use eframe::egui;
use eframe::egui::{TextEdit, Ui};
use egui_phosphor::regular::{FOLDER, FOLDER_OPEN};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Left sidebar file tree; emits navigation and file-system events. Owns the
/// last active file it drew (revealing the tree when that file changes), the
/// set of expanded directories (the app persists it per workspace) and the
/// in-place rename state.
#[derive(Default)]
pub struct FileTreePanel {
    last_active_path: Option<PathBuf>,
    /// Open directories; the app persists this per workspace.
    expanded: HashSet<PathBuf>,
    /// Path currently being renamed in place, with the new-name draft.
    renaming: Option<(PathBuf, String)>,
}

pub struct FileTreePanelInput<'a> {
    pub tree: &'a mut FileTree,
    pub active_file_path: Option<&'a Path>,
}

/// View state stable across the recursive row walk: one context instead of
/// three values threaded through every call.
struct RowContext<'a> {
    active_file_path: Option<&'a Path>,
    reveal_active_file: bool,
    viewport_width: f32,
}

impl Component for FileTreePanel {
    type Message = CustomEvent;
    type Input<'a> = FileTreePanelInput<'a>;

    fn render(&mut self, ui: &mut egui::Ui, input: Self::Input<'_>) -> Vec<CustomEvent> {
        let FileTreePanelInput {
            tree,
            active_file_path,
        } = input;
        let mut events = Vec::new();
        let reveal_active_in_tree = active_file_path != self.last_active_path.as_deref();
        if reveal_active_in_tree {
            self.last_active_path = active_file_path.map(Path::to_path_buf);
        }
        let root_entry = Entry::Directory(tree.root().to_path_buf());
        egui::Panel::left("file_tree")
            .resizable(true)
            .default_size(220.0)
            .size_range(120.0..=480.0)
            .frame(egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::ZERO))
            .show(ui, |ui| {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Stable viewport width: oversized rows grow the
                        // content's `min_rect` (which feeds the scrollbars),
                        // not `max_rect`, so `available_width()` only holds
                        // here — capture it once before any row is placed.
                        let viewport_width = ui.available_width();
                        let context = RowContext {
                            active_file_path,
                            reveal_active_file: reveal_active_in_tree,
                            viewport_width,
                        };
                        if let Some(event) = self.render_entry(ui, tree, &context, &root_entry, 0) {
                            events.push(event);
                        }
                    });
            });

        events
    }
}

impl FileTreePanel {
    /// Replace the expanded set, on workspace open.
    pub fn set_expanded(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        self.expanded = paths.into_iter().collect();
    }

    pub fn expanded(&self) -> &HashSet<PathBuf> {
        &self.expanded
    }

    fn set_expanded_for(&mut self, path: &Path, open: bool) {
        if open {
            self.expanded.insert(path.to_path_buf());
        } else {
            self.expanded.remove(path);
        }
    }

    /// Renders one row at tree depth `depth`, loading directory children
    /// from the tree on demand. While a row is being renamed the rename
    /// `TextEdit` replaces it and no row action is emitted.
    fn render_entry(
        &mut self,
        ui: &mut Ui,
        tree: &mut FileTree,
        context: &RowContext<'_>,
        entry: &Entry,
        depth: usize,
    ) -> Option<CustomEvent> {
        match entry {
            Entry::File(path) => {
                if let Some(event) = self.render_rename(ui, path, depth, |path, new_name| {
                    path.with_file_name(new_name)
                }) {
                    return Some(event);
                }
                if self.is_renaming(path) {
                    return None;
                }

                let row_height = ui.spacing().interact_size.y;
                let file_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                let is_active = context.active_file_path == Some(path.as_path());
                let (icon, icon_color) = file_type_icon(ui.visuals().dark_mode, path);
                // Widen deep rows by their indentation so nesting can overflow and scroll horizontally.
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(
                        context.viewport_width + depth as f32 * ui.spacing().indent,
                        row_height,
                    ),
                    egui::Sense::click(),
                );
                let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

                // Reveal the active row on a tab switch only when it is scrolled
                // out of the vertical viewport; never move it horizontally.
                if context.reveal_active_file && is_active {
                    let clip = ui.clip_rect();
                    let vertically_visible =
                        rect.top() >= clip.top() && rect.bottom() <= clip.bottom();
                    if !vertically_visible {
                        ui.scroll_to_rect(
                            egui::Rect::from_min_max(
                                egui::pos2(clip.left(), rect.top()),
                                egui::pos2(clip.right(), rect.bottom()),
                            ),
                            None,
                        );
                    }
                }

                if is_active || response.hovered() {
                    let bg = if is_active {
                        ui.visuals().selection.bg_fill
                    } else {
                        ui.visuals().widgets.hovered.bg_fill
                    };
                    ui.painter().rect_filled(row_fill_rect(ui, rect), 0.0, bg);
                }
                let font = egui::TextStyle::Button.resolve(ui.style());
                let left = rect.left() + depth as f32 * ui.spacing().indent;
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(left + 10.0, rect.top()),
                    egui::vec2(20.0, row_height),
                );
                ui.painter().text(
                    icon_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    icon,
                    egui::FontId::new(font.size, egui::FontFamily::Name("devicons".into())),
                    icon_color,
                );
                let text_color = if is_active {
                    ui.visuals().strong_text_color()
                } else {
                    ui.style().interact(&response).text_color()
                };
                ui.painter().text(
                    egui::pos2(icon_rect.right() + 4.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    file_name,
                    font,
                    text_color,
                );

                let mut event = None;
                if response.clicked() {
                    event = Some(CustomEvent::App(AppEvent::OpenFile(path.clone())));
                }

                response.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        self.start_renaming(path);
                        ui.close();
                    }
                    if ui.button("Delete").clicked() {
                        event = Some(CustomEvent::Workspace(WorkspaceEvent::Delete(path.clone())));
                        ui.close();
                    }
                });

                event
            }
            Entry::Directory(path) => {
                if let Some(event) = self.render_rename(ui, path, depth, |path, new_name| {
                    path.with_file_name(new_name)
                }) {
                    return Some(event);
                }
                if self.is_renaming(path) {
                    return None;
                }

                let mut event = None;

                let should_reveal = context.reveal_active_file
                    && context
                        .active_file_path
                        .is_some_and(|active_path| active_path.starts_with(path));
                if should_reveal {
                    self.set_expanded_for(path, true);
                }
                let is_open = self.expanded.contains(path);

                let row_height = ui.spacing().interact_size.y;
                let folder_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned());
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(
                        context.viewport_width + depth as f32 * ui.spacing().indent,
                        row_height,
                    ),
                    egui::Sense::click(),
                );
                let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

                if response.hovered() {
                    ui.painter().rect_filled(
                        row_fill_rect(ui, rect),
                        0.0,
                        ui.visuals().widgets.hovered.bg_fill,
                    );
                }
                let left = rect.left() + depth as f32 * ui.spacing().indent;
                let font = egui::TextStyle::Button.resolve(ui.style());
                let icon = if is_open { FOLDER_OPEN } else { FOLDER };
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(left + 10.0, rect.top()),
                    egui::vec2(20.0, row_height),
                );
                let text_color = ui.style().interact(&response).text_color();
                ui.painter().text(
                    icon_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    icon,
                    font.clone(),
                    text_color,
                );
                ui.painter().text(
                    egui::pos2(icon_rect.right() + 4.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    folder_name,
                    font,
                    text_color,
                );

                if response.clicked() {
                    self.set_expanded_for(path, !is_open);
                }

                response.context_menu(|ui| {
                    if ui.button("New File").clicked() {
                        event = Some(CustomEvent::Workspace(WorkspaceEvent::NewFile(
                            path.clone(),
                        )));
                        ui.close();
                    }
                    if ui.button("New Folder").clicked() {
                        event = Some(CustomEvent::Workspace(WorkspaceEvent::NewFolder(
                            path.clone(),
                        )));
                        ui.close();
                    }
                    if ui.button("Rename").clicked() {
                        self.start_renaming(path);
                        ui.close();
                    }
                    if ui.button("Delete").clicked() {
                        event = Some(CustomEvent::Workspace(WorkspaceEvent::Delete(path.clone())));
                        ui.close();
                    }
                });

                if is_open {
                    let children = tree.children(path);
                    for entry in children.iter() {
                        if let Some(child_event) =
                            self.render_entry(ui, tree, context, entry, depth + 1)
                        {
                            event = Some(child_event);
                        }
                    }
                }

                event
            }
        }
    }

    /// In-place rename editor for the row at `path`. Returns the rename event
    /// once committed, `None` while cancelled/absent.
    fn render_rename(
        &mut self,
        ui: &mut Ui,
        path: &Path,
        depth: usize,
        build_new_path: impl FnOnce(&Path, &str) -> PathBuf,
    ) -> Option<CustomEvent> {
        if !self.is_renaming(path) {
            return None;
        }
        ui.add_space(depth as f32 * ui.spacing().indent);
        let (_, new_name) = self.renaming.as_mut().expect("renaming checked above");
        let response = ui.add(TextEdit::singleline(new_name).hint_text("New name..."));
        // When the tree is horizontally scrolled the field can start off
        // screen; nudge the viewport to keep it (partially) visible. No-op
        // while the field is already fully in view.
        response.scroll_to_me(None);
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let event = CustomEvent::Workspace(WorkspaceEvent::Rename(
                path.to_path_buf(),
                build_new_path(path, new_name),
            ));
            self.renaming = None;
            return Some(event);
        }
        if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.renaming = None;
        }
        None
    }

    fn is_renaming(&self, path: &Path) -> bool {
        self.renaming
            .as_ref()
            .is_some_and(|(renaming_path, _)| renaming_path == path)
    }

    fn start_renaming(&mut self, path: &Path) {
        self.renaming = Some((
            path.to_path_buf(),
            path.file_name().unwrap().to_string_lossy().to_string(),
        ));
    }
}

/// Background span for a row: cover the visible viewport even when a shallow
/// row is narrower than the horizontal scroll range.
fn row_fill_rect(ui: &Ui, rect: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.top()),
        egui::pos2(rect.right().max(ui.clip_rect().right()), rect.bottom()),
    )
}
