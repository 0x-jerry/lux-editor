use crate::app::App;
use crate::events::{CustomEvent, DocumentEvent};
use eframe::egui;
use std::io::Write;

impl App {
    /// Format the active document by piping it through the configured external
    /// formatter. The result is applied only if the buffer generation still
    /// matches (i.e. the user has not typed since the request was launched).
    pub(crate) fn format_active_document(&mut self, _ctx: &egui::Context) {
        let formatter = self.settings.editor_config.settings.formatter.clone();
        if formatter.command.trim().is_empty() {
            self.active_document_mut().document_status =
                Some("No formatter configured".to_string());
            return;
        }

        let text = self.active_document().buffer.text().to_string();
        let generation = self.active_document().edit_generation;
        let event_tx = self.runtime.event_tx.clone();
        let wake = self.runtime.ctx.clone();
        self.active_document_mut().document_status = Some("Formatting…".to_string());

        self.runtime.rt.spawn_blocking(move || {
            let result = run_formatter(&formatter.command, &formatter.args, &text);
            let _ = event_tx.send(CustomEvent::Document(DocumentEvent::FormattingFinished {
                generation,
                from_save: false,
                result,
            }));
            wake.request_repaint();
        });
    }

    pub(crate) fn on_formatting_finished(
        &mut self,
        generation: u64,
        from_save: bool,
        result: Result<String, String>,
        ctx: &egui::Context,
    ) {
        // The buffer moved on while the formatter was running; the result is
        // stale and must not clobber newer edits.
        if self.active_document().edit_generation != generation {
            return;
        }

        match result {
            Ok(formatted) => {
                if formatted
                    .chars()
                    .eq(self.active_document().buffer.text().chars())
                {
                    self.active_document_mut().document_status =
                        Some("Already formatted".to_string());
                    return;
                }
                let total_chars = self.active_document().buffer.text().len_chars();
                let positions = self.active_document().caret_state.caret_chars_snapshot();
                if self.apply_edit(0, total_chars, &formatted, ctx) {
                    let active_document = self.active_document_mut();
                    let len = active_document.buffer.text().len_chars();
                    let clamped = positions
                        .iter()
                        .map(|pos| (*pos).min(len))
                        .collect::<Vec<usize>>();
                    active_document
                        .caret_state
                        .set_all_caret_chars(&clamped, &active_document.buffer);
                    if from_save {
                        // The save task already wrote this text to disk, so the
                        // buffer is in the saved state, not dirty.
                        active_document.document_dirty = false;
                        active_document.document_status = Some("Formatted".to_string());
                    } else {
                        active_document.document_status = Some("Formatted".to_string());
                    }
                    self.schedule_language_refresh();
                }
            }
            Err(err) => {
                self.active_document_mut().document_status =
                    Some(format!("Format failed: {}", err));
            }
        }
    }
}

/// Run `command args` with `text` on stdin and return stdout. The formatter is
/// expected to be a stdin/stdout filter (e.g. `oxfmt --stdin`, `rustfmt`).
pub(crate) fn run_formatter(command: &str, args: &str, text: &str) -> Result<String, String> {
    let mut cmd = std::process::Command::new(command);
    for arg in args.split_whitespace() {
        cmd.arg(arg);
    }
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let mut child = cmd.spawn().map_err(|err| err.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|err| err.to_string())?;
    }
    let output = child.wait_with_output().map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(format!("formatter exited with status {}", output.status));
    }
    String::from_utf8(output.stdout).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::run_formatter;

    #[test]
    fn cat_passthrough_formats_identity() {
        let result = run_formatter("cat", "", "hello\nworld").unwrap();
        assert_eq!(result, "hello\nworld");
    }

    #[test]
    fn missing_command_errors() {
        let result = run_formatter("lux-formatter-does-not-exist", "", "x");
        assert!(result.is_err());
    }

    #[test]
    fn args_are_split_on_whitespace() {
        // `printf %s` with a format argument echoes stdin verbatim.
        let result = run_formatter("cat", "", "ab").unwrap();
        assert_eq!(result, "ab");
    }

    #[test]
    fn non_zero_exit_is_an_error() {
        let result = run_formatter("sh", "-c 'exit 3'", "x");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("status"));
    }
}
