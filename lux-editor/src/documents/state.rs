//! Document domain: the open tabs and the caret-blink state scoped to them.

use crate::app::App;
use crate::documents::OpenDocument;
use crate::documents::formatter::run_formatter;
use crate::events::{CustomEvent, DocumentEvent};
use eframe::egui;
use lux_core::Buffer;
use std::path::PathBuf;
use std::time::Instant;

pub(crate) struct Documents {
    pub(crate) tabs: Vec<OpenDocument>,
    pub(crate) active_document: usize,
    pub(crate) caret_blink_anchor: Instant,
}

impl Documents {
    const CARET_BLINK_PERIOD: std::time::Duration = std::time::Duration::from_millis(1000);

    pub(crate) fn with_empty_document() -> Self {
        Self {
            tabs: vec![OpenDocument::new_empty()],
            active_document: 0,
            caret_blink_anchor: Instant::now(),
        }
    }

    pub(crate) fn active_document(&self) -> &OpenDocument {
        self.tabs
            .get(self.active_document)
            .expect("active document index must be valid")
    }

    pub(crate) fn active_document_mut(&mut self) -> &mut OpenDocument {
        self.tabs
            .get_mut(self.active_document)
            .expect("active document index must be valid")
    }

    pub(crate) fn reset_editor_state(&mut self) {
        let active_document = self.active_document_mut();
        active_document
            .caret_state
            .reset_to_buffer_end(&active_document.buffer);
        active_document.edit_history.clear();
        self.touch_caret_blink();
    }

    pub(crate) fn caret_blink_visible(&self) -> bool {
        self.caret_blink_anchor.elapsed().as_millis() % Self::CARET_BLINK_PERIOD.as_millis()
            < (Self::CARET_BLINK_PERIOD.as_millis() / 2)
    }

    pub(crate) fn touch_caret_blink(&mut self) {
        self.caret_blink_anchor = std::time::Instant::now();
    }
}

impl App {
    pub(crate) fn open_file(&mut self, path: PathBuf, ctx: &egui::Context) {
        let path = path.canonicalize().unwrap_or(path);
        if let Some(index) = self.documents.tabs.iter().position(|doc| {
            doc.buffer
                .path()
                .is_some_and(|existing_path| existing_path == &path)
        }) {
            self.switch_to_document(index, ctx);
            return;
        }

        let event_tx = self.runtime.event_tx.clone();
        let wake = self.runtime.ctx.clone();
        self.runtime.rt.spawn(async move {
            let buffer = Buffer::from_file(&path)
                .await
                .map_err(|err| err.to_string());
            let _ = event_tx.send(CustomEvent::Document(DocumentEvent::FileLoaded {
                path,
                buffer,
            }));
            wake.request_repaint();
        });
    }

    pub(crate) fn save_current_buffer(&mut self, _ctx: &egui::Context) -> bool {
        let active_path = self.active_document().buffer.path().cloned();
        if active_path.is_none() {
            if let Some(path) = rfd::FileDialog::new().save_file() {
                self.buffer_mut().set_path(&path);
            } else {
                self.active_document_mut().document_status = Some("Save cancelled".to_string());
                return false;
            }
        }

        let save_path = self.buffer().path().cloned().unwrap();
        let text = self.buffer().text().to_string();
        let generation = self.active_document().edit_generation;
        let formatter = self.settings.editor_config.settings.formatter.clone();
        let format_on_save = formatter.format_on_save && !formatter.command.trim().is_empty();
        let event_tx = self.runtime.event_tx.clone();
        let wake = self.runtime.ctx.clone();
        self.runtime.rt.spawn_blocking(move || {
            let mut to_write = text.clone();
            let mut formatted_result: Option<Result<String, String>> = None;
            if format_on_save {
                match run_formatter(&formatter.command, &formatter.args, &text) {
                    Ok(formatted) if formatted != text => {
                        to_write = formatted.clone();
                        formatted_result = Some(Ok(formatted));
                    }
                    Ok(_) => {}
                    Err(err) => formatted_result = Some(Err(err)),
                }
            }
            let ok = std::fs::write(&save_path, to_write).is_ok();
            let _ = event_tx.send(CustomEvent::Document(DocumentEvent::FileSaved {
                path: save_path,
                generation,
                ok,
            }));
            if let Some(result) = formatted_result {
                let _ = event_tx.send(CustomEvent::Document(DocumentEvent::FormattingFinished {
                    generation,
                    from_save: true,
                    result,
                }));
            }
            wake.request_repaint();
        });
        true
    }

    pub(crate) fn update_window_title(&self, ctx: &egui::Context) {
        let active_document = self.active_document();
        let title = if let Some(path) = active_document.buffer.path() {
            let dirty_prefix = if active_document.document_dirty {
                "* "
            } else {
                ""
            };
            format!("lux - {}{}", dirty_prefix, path.display())
        } else if active_document.document_dirty {
            "lux - * Untitled".to_string()
        } else {
            "lux".to_string()
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }

    pub(crate) fn switch_to_document(&mut self, index: usize, ctx: &egui::Context) {
        if index >= self.documents.tabs.len() {
            return;
        }
        self.documents.active_document = index;
        self.documents.touch_caret_blink();
        self.update_window_title(ctx);
        self.refresh_language_intelligence();
    }

    pub(crate) fn close_document(&mut self, index: usize, ctx: &egui::Context) {
        if index >= self.documents.tabs.len() {
            return;
        }

        if self.documents.tabs[index].document_dirty {
            self.documents.tabs[index].document_status =
                Some("Unsaved changes — save before closing".to_string());
            return;
        }

        if self.documents.tabs.len() == 1 {
            self.documents.tabs[0] = OpenDocument::new_empty();
            self.documents.active_document = 0;
            self.documents.touch_caret_blink();
            self.update_window_title(ctx);
            self.refresh_language_intelligence();
            return;
        }

        self.documents.tabs.remove(index);
        if self.documents.active_document >= self.documents.tabs.len() {
            self.documents.active_document = self.documents.tabs.len().saturating_sub(1);
        } else if index < self.documents.active_document {
            self.documents.active_document = self.documents.active_document.saturating_sub(1);
        }
        self.documents.touch_caret_blink();
        self.update_window_title(ctx);
        self.refresh_language_intelligence();
    }

    pub(crate) fn should_reuse_active_document_slot(&self) -> bool {
        if self.documents.tabs.len() != 1 || self.documents.active_document != 0 {
            return false;
        }
        let active_document = self.active_document();
        active_document.buffer.path().is_none()
            && !active_document.document_dirty
            && active_document.buffer.text().len_chars() == 0
    }
}
