use crate::chrome::ui::widgets::{file_type_icon, prompt_frame};
use crate::component::Component;
use crate::events::{AppEvent, CustomEvent, WorkspaceEvent};
use crate::workspace::{Entry, FileTree};
use eframe::egui;
use eframe::egui::{TextEdit, Ui};
use egui_phosphor::regular::{FOLDER, FOLDER_OPEN, WARNING_CIRCLE};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Left sidebar file tree; emits navigation and file-system events. Owns the
/// last active file it drew (revealing the tree when that file changes), the
/// set of expanded directories (the app persists it per workspace), the
/// in-place rename state and the pending delete confirmation.
#[derive(Default)]
pub struct FileTreePanel {
    last_active_path: Option<PathBuf>,
    /// Open directories; the app persists this per workspace.
    expanded: HashSet<PathBuf>,
    /// Path currently being renamed in place, with the new-name draft.
    renaming: Option<(PathBuf, String)>,
    /// Path a Delete click is waiting on; the confirmation modal decides.
    confirm_delete: Option<PathBuf>,
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
        let root_entry = Entry::Directory {
            path: tree.root().to_path_buf(),
            ignored: false,
        };
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

        events.extend(self.render_delete_confirm(ui));
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
            Entry::File { path, ignored } => {
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
                // A gitignored row reads as dimmed: weak gray for glyph and
                // label, over both the devicons brand color and the row state.
                let dim = (*ignored).then(|| ui.visuals().weak_text_color());
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
                    dim.unwrap_or(icon_color),
                );
                let text_color = dim.unwrap_or_else(|| {
                    if is_active {
                        ui.visuals().strong_text_color()
                    } else {
                        ui.style().interact(&response).text_color()
                    }
                });
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
                        self.confirm_delete = Some(path.clone());
                        ui.close();
                    }
                });

                event
            }
            Entry::Directory { path, ignored } => {
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
                let text_color = if *ignored {
                    ui.visuals().weak_text_color()
                } else {
                    ui.style().interact(&response).text_color()
                };
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
                    // The root row is the whole workspace: renaming or deleting
                    // it is never what a click on the top line means.
                    if depth > 0 {
                        if ui.button("Rename").clicked() {
                            self.start_renaming(path);
                            ui.close();
                        }
                        if ui.button("Delete").clicked() {
                            self.confirm_delete = Some(path.clone());
                            ui.close();
                        }
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

    /// Confirmation for a queued delete: the tree has no undo, so a directory
    /// removal is one modal away from the click that asked for it.
    fn render_delete_confirm(&mut self, ui: &mut Ui) -> Option<CustomEvent> {
        let path = self.confirm_delete.clone()?;
        if path.symlink_metadata().is_err() {
            // It went away (or was replaced) since the menu was clicked.
            self.confirm_delete = None;
            return None;
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let is_dir = path.is_dir();
        let mut confirmed = false;

        let modal = egui::Modal::new(egui::Id::new("file_tree_delete"))
            .frame(prompt_frame(ui))
            .show(ui.ctx(), |ui| {
                ui.set_min_width(360.0);
                ui.set_max_width(360.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(WARNING_CIRCLE)
                            .size(20.0)
                            .color(ui.visuals().error_fg_color),
                    );
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Delete").size(16.0).strong());
                });
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Delete {} \u{201c}{}\u{201d}?",
                        if is_dir { "this folder" } else { "this file" },
                        name
                    ))
                    .size(13.0),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(if is_dir {
                        "Everything inside it goes too. This cannot be undone."
                    } else {
                        "This cannot be undone."
                    })
                    .size(12.0)
                    .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let delete = ui.add(
                        egui::Button::new(
                            egui::RichText::new("Delete").color(ui.visuals().error_fg_color),
                        )
                        .min_size(egui::vec2(88.0, 30.0)),
                    );
                    if delete.clicked() {
                        confirmed = true;
                    }
                    if ui
                        .add(egui::Button::new("Cancel").min_size(egui::vec2(88.0, 30.0)))
                        .clicked()
                    {
                        self.confirm_delete = None;
                    }
                });
            });

        // Click-away and Escape cancel; Enter must not, it is the key that
        // confirms things elsewhere.
        if modal.should_close() {
            self.confirm_delete = None;
        }
        if confirmed {
            self.confirm_delete = None;
            return Some(CustomEvent::Workspace(WorkspaceEvent::Delete(path)));
        }
        None
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
        // A filesystem root has no file name; the row renders its full path.
        let draft = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        self.renaming = Some((path.to_path_buf(), draft));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `(text, color)` of every glyph run in the frame, flattened.
    fn painted(shape: &egui::Shape, into: &mut Vec<(String, egui::Color32)>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| painted(shape, into)),
            egui::Shape::Text(text) => {
                into.push((text.galley.text().to_owned(), text.fallback_color))
            }
            _ => {}
        }
    }

    /// The delete confirmation paints the target's name and removes nothing on
    /// its own: the row menu only arms the modal.
    #[test]
    fn delete_confirmation_paints_the_target_and_asks_first() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("doomed.rs"), "").unwrap();

        let mut tree = FileTree::new(root.path(), &[]);
        let mut panel = FileTreePanel::default();
        panel.set_expanded([root.path().to_path_buf()]);
        panel.confirm_delete = Some(root.path().join("doomed.rs"));

        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "devicons".into(),
            egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/SymbolsNerdFontMono-Regular.ttf"
            ))
            .into(),
        );
        fonts.families.insert(
            egui::FontFamily::Name("devicons".into()),
            vec!["devicons".into()],
        );
        ctx.set_fonts(fonts);

        let mut events = Vec::new();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                ..Default::default()
            },
            |ui| {
                events = panel.render(
                    ui,
                    FileTreePanelInput {
                        tree: &mut tree,
                        active_file_path: None,
                    },
                );
            },
        );
        output.textures_delta.clear();

        let mut rows = vec![];
        output
            .shapes
            .iter()
            .for_each(|clipped| painted(&clipped.shape, &mut rows));
        assert!(
            rows.iter().any(|(text, _)| text.contains("doomed.rs")),
            "the confirmation names the file: {rows:?}"
        );
        assert!(events.is_empty(), "opening the prompt deletes nothing");
        assert!(
            panel.confirm_delete.is_some(),
            "it stays armed until answered"
        );
        assert!(root.path().join("doomed.rs").exists());
    }

    /// One frame of the sidebar, painted headlessly: every row is a glyph run
    /// plus a label, so what reached the painter is what the user sees.
    #[test]
    fn excluded_rows_never_paint_and_ignored_rows_paint_dim() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path();
        std::fs::write(path.join(".gitignore"), "*.log\n").unwrap();
        std::fs::create_dir(path.join(".git")).unwrap();
        std::fs::write(path.join("notes.log"), "").unwrap();
        std::fs::write(path.join("main.rs"), "").unwrap();

        let mut tree = FileTree::new(path, &[".git".to_string()]);
        let mut panel = FileTreePanel::default();
        panel.set_expanded([path.to_path_buf()]);
        let ctx = egui::Context::default();
        // File glyphs are laid out in the app's devicons family; bind it so a
        // headless frame can measure text.
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "devicons".into(),
            egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/SymbolsNerdFontMono-Regular.ttf"
            ))
            .into(),
        );
        fonts.families.insert(
            egui::FontFamily::Name("devicons".into()),
            vec!["devicons".into()],
        );
        ctx.set_fonts(fonts);
        let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let mut weak = egui::Color32::BLACK;
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ui| {
                weak = ui.visuals().weak_text_color();
                panel.render(
                    ui,
                    FileTreePanelInput {
                        tree: &mut tree,
                        active_file_path: None,
                    },
                );
            },
        );

        // Nothing paints this frame; drop the font-atlas delta deliberately.
        output.textures_delta.clear();

        let mut rows = vec![];
        output
            .shapes
            .iter()
            .for_each(|clipped| painted(&clipped.shape, &mut rows));
        let color_of = |name: &str| {
            rows.iter()
                .find(|(text, _)| text == name)
                .unwrap_or_else(|| panic!("{name} was not painted"))
                .1
        };
        assert_eq!(color_of("notes.log"), weak, "gitignored rows are dimmed");
        assert_ne!(color_of("main.rs"), weak, "other rows keep their color");
        assert!(
            !rows.iter().any(|(text, _)| text == ".git"),
            "excluded entries never reach the panel"
        );
    }
}
