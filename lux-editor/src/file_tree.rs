
use egui::{collapsing_header::CollapsingState, Id, Ui, TextEdit};
use std::path::{Path, PathBuf};
use winit::event_loop::EventLoopProxy;
use crate::CustomEvent;
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
    event_proxy: EventLoopProxy<CustomEvent>,
}

impl FileTree {
    pub fn new(path: &Path, event_proxy: EventLoopProxy<CustomEvent>) -> Self {
        Self { entry: Self::build_entry(path), event_proxy }
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
                            let response = ui.add(TextEdit::singleline(new_name).hint_text("New name..."));
                            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                let new_path = path.with_file_name(&*new_name);
                                self.event_proxy.send_event(CustomEvent::Rename(path.clone(), new_path)).ok();
                                *renaming = None;
                            } else if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
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

                let response = ui.with_layout(egui::Layout::left_to_right(egui::Align::Center).with_main_justify(true), |ui| {
                    ui.selectable_label(false, path.file_name().unwrap().to_str().unwrap())
                }).inner;
                
                let mut clicked_path = None;
                if response.clicked() {
                    clicked_path = Some(path.clone());
                }

                response.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        let mut renaming = RENAMING_STATE.lock().unwrap();
                        *renaming = Some((path.clone(), path.file_name().unwrap().to_string_lossy().to_string()));
                        ui.close_menu();
                    }
                    if ui.button("Delete").clicked() {
                        self.event_proxy.send_event(CustomEvent::Delete(path.clone())).ok();
                        ui.close_menu();
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
                            let response = ui.add(TextEdit::singleline(new_name).hint_text("New name..."));
                            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                let new_path = path.with_file_name(&*new_name);
                                self.event_proxy.send_event(CustomEvent::Rename(path.clone(), new_path)).ok();
                                *renaming = None;
                            } else if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
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

                let is_open = CollapsingState::load_with_default_open(ui.ctx(), id, false).is_open();
                CollapsingState::load_with_default_open(ui.ctx(), id, false)
                    .show_header(ui, |ui| {
                        let response = ui.with_layout(egui::Layout::left_to_right(egui::Align::Center).with_main_justify(true), |ui| {
                            ui.selectable_label(false, path.file_name().unwrap_or_default().to_string_lossy().to_string())
                        }).inner;
                        
                        if response.clicked() {
                            let mut state = CollapsingState::load_with_default_open(ui.ctx(), id, false);
                            state.set_open(!is_open);
                            state.store(ui.ctx());
                        }

                        response.context_menu(|ui| {
                            if ui.button("New File").clicked() {
                                self.event_proxy.send_event(CustomEvent::NewFile(path.clone())).ok();
                                ui.close_menu();
                            }
                            if ui.button("New Folder").clicked() {
                                self.event_proxy.send_event(CustomEvent::NewFolder(path.clone())).ok();
                                ui.close_menu();
                            }
                            if ui.button("Rename").clicked() {
                                let mut renaming = RENAMING_STATE.lock().unwrap();
                                *renaming = Some((path.clone(), path.file_name().unwrap().to_string_lossy().to_string()));
                                ui.close_menu();
                            }
                            if ui.button("Delete").clicked() {
                                self.event_proxy.send_event(CustomEvent::Delete(path.clone())).ok();
                                ui.close_menu();
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
            for entry in path.read_dir().expect("read_dir call failed") {
                if let Ok(entry) = entry {
                    entries.push(Self::build_entry(&entry.path()));
                }
            }
            Entry::Directory(path.to_path_buf(), entries)
        } else {
            Entry::File(path.to_path_buf())
        }
    }
}
