use crate::events::CustomEvent;
use egui::{Id, TextEdit, Ui, collapsing_header::CollapsingState};
use egui_phosphor::regular::{FILE_CODE, FOLDER, FOLDER_OPEN};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref RENAMING_STATE: Mutex<Option<(PathBuf, String)>> = Mutex::new(None);
}

#[derive(Clone)]
pub enum Entry {
    File(PathBuf),
    Directory(PathBuf, Vec<Entry>),
}

pub struct FileTree {
    entry: Entry,
}

impl FileTree {
    pub fn new(path: &Path) -> Self {
        let ignored = Self::build_gitignore(path);
        Self {
            entry: Self::build_entry(path, &ignored),
        }
    }

    pub fn show(
        &self,
        ui: &mut Ui,
        active_file_path: Option<&Path>,
        reveal_active_file: bool,
    ) -> Option<CustomEvent> {
        self.show_entry(ui, &self.entry, active_file_path, reveal_active_file)
    }

    fn show_entry(
        &self,
        ui: &mut Ui,
        entry: &Entry,
        active_file_path: Option<&Path>,
        reveal_active_file: bool,
    ) -> Option<CustomEvent> {
        match entry {
            Entry::File(path) => {
                let is_renaming = {
                    let mut renaming = RENAMING_STATE.lock().unwrap();
                    if let Some((renaming_path, new_name)) = renaming.as_mut() {
                        if renaming_path == path {
                            let response =
                                ui.add(TextEdit::singleline(new_name).hint_text("New name..."));
                            if response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                let new_path = path.with_file_name(&*new_name);
                                let event = CustomEvent::Rename(path.clone(), new_path);
                                *renaming = None;
                                return Some(event);
                            }
                            if response.lost_focus()
                                || ui.input(|i| i.key_pressed(egui::Key::Escape))
                            {
                                *renaming = None;
                            }
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };

                if is_renaming {
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
                    event = Some(CustomEvent::OpenFile(path.clone()));
                }

                response.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        let mut renaming = RENAMING_STATE.lock().unwrap();
                        *renaming = Some((
                            path.clone(),
                            path.file_name().unwrap().to_string_lossy().to_string(),
                        ));
                        ui.close();
                    }
                    if ui.button("Delete").clicked() {
                        event = Some(CustomEvent::Delete(path.clone()));
                        ui.close();
                    }
                });

                event
            }
            Entry::Directory(path, entries) => {
                let id = Id::new(path);
                let mut event = None;

                let is_renaming = {
                    let mut renaming = RENAMING_STATE.lock().unwrap();
                    if let Some((renaming_path, new_name)) = renaming.as_mut() {
                        if renaming_path == path {
                            let response =
                                ui.add(TextEdit::singleline(new_name).hint_text("New name..."));
                            if response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                let new_path = path.with_file_name(&*new_name);
                                let rename_event = CustomEvent::Rename(path.clone(), new_path);
                                *renaming = None;
                                return Some(rename_event);
                            }
                            if response.lost_focus()
                                || ui.input(|i| i.key_pressed(egui::Key::Escape))
                            {
                                *renaming = None;
                            }
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };

                if is_renaming {
                    return None;
                }

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
                                event = Some(CustomEvent::NewFile(path.clone()));
                                ui.close();
                            }
                            if ui.button("New Folder").clicked() {
                                event = Some(CustomEvent::NewFolder(path.clone()));
                                ui.close();
                            }
                            if ui.button("Rename").clicked() {
                                let mut renaming = RENAMING_STATE.lock().unwrap();
                                *renaming = Some((
                                    path.clone(),
                                    path.file_name().unwrap().to_string_lossy().to_string(),
                                ));
                                ui.close();
                            }
                            if ui.button("Delete").clicked() {
                                event = Some(CustomEvent::Delete(path.clone()));
                                ui.close();
                            }
                        });
                    })
                    .body(|ui| {
                        for entry in entries {
                            if let Some(child_event) =
                                self.show_entry(ui, entry, active_file_path, reveal_active_file)
                            {
                                event = Some(child_event);
                            }
                        }
                    });

                event
            }
        }
    }

    fn build_entry(path: &Path, ignored: &Gitignore) -> Entry {
        if path.is_dir() {
            let mut entries = vec![];
            for entry in path.read_dir().expect("read_dir call failed").flatten() {
                let entry_path = entry.path();
                let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
                if ignored
                    .matched_path_or_any_parents(&entry_path, is_dir)
                    .is_ignore()
                {
                    continue;
                }
                entries.push(Self::build_entry(&entry_path, ignored));
            }
            entries.sort_by(|a, b| {
                let a_is_dir = matches!(a, Entry::Directory(_, _));
                let b_is_dir = matches!(b, Entry::Directory(_, _));
                if a_is_dir != b_is_dir {
                    return b_is_dir.cmp(&a_is_dir);
                }
                let a_name = Self::entry_name(a).to_lowercase();
                let b_name = Self::entry_name(b).to_lowercase();
                a_name.cmp(&b_name)
            });
            Entry::Directory(path.to_path_buf(), entries)
        } else {
            Entry::File(path.to_path_buf())
        }
    }

    fn build_gitignore(root: &Path) -> Gitignore {
        let mut builder = GitignoreBuilder::new(root);
        let gitignore_path = root.join(".gitignore");
        if gitignore_path.exists() {
            builder.add(gitignore_path);
        }
        builder.build().unwrap_or_else(|_| {
            let fallback = GitignoreBuilder::new(root);
            fallback.build().expect("gitignore builder must succeed")
        })
    }

    fn entry_name(entry: &Entry) -> String {
        match entry {
            Entry::File(path) => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            Entry::Directory(path, _) => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }
}
