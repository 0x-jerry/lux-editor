use crate::events::CustomEvent;
use egui::{Id, TextEdit, Ui, collapsing_header::CollapsingState};
use egui_phosphor::regular::{FILE_CODE, FOLDER, FOLDER_OPEN};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use winit::event_loop::EventLoopProxy;

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
    event_proxy: EventLoopProxy<CustomEvent>,
}

impl FileTree {
    pub fn new(path: &Path, event_proxy: EventLoopProxy<CustomEvent>) -> Self {
        Self {
            entry: Self::build_entry(path),
            event_proxy,
        }
    }

    pub fn show(&self, ui: &mut Ui) -> Option<PathBuf> {
        self.show_entry(ui, &self.entry)
    }

    fn show_entry(&self, ui: &mut Ui, entry: &Entry) -> Option<PathBuf> {
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
                                self.event_proxy
                                    .send_event(CustomEvent::Rename(path.clone(), new_path))
                                    .ok();
                                *renaming = None;
                            } else if response.lost_focus()
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
                let response = ui
                    .allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), row_height),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| ui.selectable_label(false, format!("{} {}", FILE_CODE, file_name)),
                    )
                    .inner;

                let mut clicked_path = None;
                if response.clicked() {
                    clicked_path = Some(path.clone());
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
                        self.event_proxy
                            .send_event(CustomEvent::Delete(path.clone()))
                            .ok();
                        ui.close();
                    }
                });

                clicked_path
            }
            Entry::Directory(path, entries) => {
                let id = Id::new(path);
                let mut clicked_path = None;

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
                                self.event_proxy
                                    .send_event(CustomEvent::Rename(path.clone(), new_path))
                                    .ok();
                                *renaming = None;
                            } else if response.lost_focus()
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

                let is_open =
                    CollapsingState::load_with_default_open(ui.ctx(), id, false).is_open();
                CollapsingState::load_with_default_open(ui.ctx(), id, false)
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
                            let mut state =
                                CollapsingState::load_with_default_open(ui.ctx(), id, false);
                            state.set_open(!is_open);
                            state.store(ui.ctx());
                        }

                        response.context_menu(|ui| {
                            if ui.button("New File").clicked() {
                                self.event_proxy
                                    .send_event(CustomEvent::NewFile(path.clone()))
                                    .ok();
                                ui.close();
                            }
                            if ui.button("New Folder").clicked() {
                                self.event_proxy
                                    .send_event(CustomEvent::NewFolder(path.clone()))
                                    .ok();
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
                                self.event_proxy
                                    .send_event(CustomEvent::Delete(path.clone()))
                                    .ok();
                                ui.close();
                            }
                        });
                    })
                    .body(|ui| {
                        for entry in entries {
                            if let Some(p) = self.show_entry(ui, entry) {
                                clicked_path = Some(p);
                            }
                        }
                    });

                clicked_path
            }
        }
    }

    fn build_entry(path: &Path) -> Entry {
        if path.is_dir() {
            let mut entries = vec![];
            for entry in path.read_dir().expect("read_dir call failed").flatten() {
                entries.push(Self::build_entry(&entry.path()));
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
