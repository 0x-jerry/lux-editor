use crate::events::{AppEvent, CustomEvent, WorkspaceEvent};
use crate::workspace::{Entry, FileTree};
use crate::component::Component;
use eframe::egui;
use eframe::egui::{Id, TextEdit, Ui, collapsing_header::CollapsingState};
use egui_phosphor::regular::{FILE_CODE, FOLDER, FOLDER_OPEN};
use std::path::{Path, PathBuf};

/// Left sidebar file tree; emits navigation and file-system events. Owns the
/// last active file it drew (revealing the tree when that file changes) and
/// the in-place rename state.
#[derive(Default)]
pub struct FileTreePanel {
    last_active_path: Option<PathBuf>,
    /// Path currently being renamed in place, with the new-name draft.
    renaming: Option<(PathBuf, String)>,
}

pub struct FileTreePanelInput<'a> {
    pub tree: &'a mut FileTree,
    pub active_file_path: Option<&'a Path>,
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
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(event) = self.render_entry(
                            ui,
                            tree,
                            &root_entry,
                            active_file_path,
                            reveal_active_in_tree,
                        ) {
                            events.push(event);
                        }
                    });
            });

        events
    }
}

impl FileTreePanel {
    /// Renders one row, loading directory children from the tree on demand.
    /// While a row is being renamed the rename `TextEdit` replaces it and no
    /// row action is emitted.
    fn render_entry(
        &mut self,
        ui: &mut Ui,
        tree: &mut FileTree,
        entry: &Entry,
        active_file_path: Option<&Path>,
        reveal_active_file: bool,
    ) -> Option<CustomEvent> {
        match entry {
            Entry::File(path) => {
                if let Some(event) =
                    self.render_rename(ui, path, |path, new_name| path.with_file_name(new_name))
                {
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
                let is_active = active_file_path == Some(path.as_path());
                let response = ui
                    .allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), row_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| ui.selectable_label(is_active, format!("{} {}", FILE_CODE, file_name)),
                    )
                    .inner;

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
                if let Some(event) =
                    self.render_rename(ui, path, |path, new_name| path.with_file_name(new_name))
                {
                    return Some(event);
                }
                if self.is_renaming(path) {
                    return None;
                }

                let id = Id::new(path);
                let mut event = None;

                let should_reveal = reveal_active_file
                    && active_file_path.is_some_and(|active_path| active_path.starts_with(path));
                let mut state =
                    CollapsingState::load_with_default_open(ui.ctx(), id, should_reveal);
                if should_reveal && !state.is_open() {
                    state.set_open(true);
                    state.store(ui.ctx());
                }
                let is_open = state.is_open();
                state
                    .show_header(ui, |ui| {
                        let row_height = ui.spacing().interact_size.y;
                        let folder_name = path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.to_string_lossy().into_owned());
                        let icon = if is_open { FOLDER_OPEN } else { FOLDER };
                        let response = ui
                            .allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), row_height),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.selectable_label(false, format!("{} {}", icon, folder_name))
                                },
                            )
                            .inner;

                        if response.clicked() {
                            let mut state = CollapsingState::load_with_default_open(
                                ui.ctx(),
                                id,
                                should_reveal,
                            );
                            state.set_open(!is_open);
                            state.store(ui.ctx());
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
                                event = Some(CustomEvent::Workspace(WorkspaceEvent::Delete(
                                    path.clone(),
                                )));
                                ui.close();
                            }
                        });
                    })
                    .body(|ui| {
                        let children = tree.children(path);
                        for entry in children.iter() {
                            if let Some(child_event) =
                                self.render_entry(ui, tree, entry, active_file_path, reveal_active_file)
                            {
                                event = Some(child_event);
                            }
                        }
                    });

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
        build_new_path: impl FnOnce(&Path, &str) -> PathBuf,
    ) -> Option<CustomEvent> {
        if !self.is_renaming(path) {
            return None;
        }
        let (_, new_name) = self.renaming.as_mut().expect("renaming checked above");
        let response = ui.add(TextEdit::singleline(new_name).hint_text("New name..."));
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
