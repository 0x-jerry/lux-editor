use super::App;
use eframe::egui;
use lux_core::Buffer;

impl App {
    pub(super) fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }

        let mut changed = false;
        let events = ctx.input(|input| input.events.clone());
        for event in events {
            match event {
                egui::Event::Text(text) => {
                    if !text.starts_with(|c: char| c.is_ascii_control()) {
                        let char_idx = self.buffer.text().len_chars();
                        self.buffer.insert(char_idx, &text);
                        changed = true;
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if modifiers.ctrl || modifiers.command || modifiers.alt {
                        continue;
                    }
                    match key {
                        egui::Key::Enter => {
                            let indentation = Self::indentation_for_newline(&self.buffer);
                            let char_idx = self.buffer.text().len_chars();
                            self.buffer.insert(char_idx, &indentation);
                            changed = true;
                        }
                        egui::Key::Backspace => {
                            let char_idx = self.buffer.text().len_chars();
                            if char_idx > 0 {
                                self.buffer.remove(char_idx - 1..char_idx);
                                changed = true;
                            }
                        }
                        egui::Key::Tab => {
                            let char_idx = self.buffer.text().len_chars();
                            self.buffer.insert(char_idx, "    ");
                            changed = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        if changed {
            self.refresh_language_intelligence();
        }
    }

    fn indentation_for_newline(buffer: &Buffer) -> String {
        const INDENT: &str = "    ";

        let total_chars = buffer.text().len_chars();
        if total_chars == 0 {
            return "\n".to_string();
        }

        let line_idx = buffer.text().char_to_line(total_chars.saturating_sub(1));
        let line = buffer.text().line(line_idx).to_string();
        let content = line.trim_end_matches(['\n', '\r']);
        let leading = content
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect::<String>();
        let trimmed = content.trim_end();

        if trimmed.ends_with('{') {
            return format!("\n{}{}", leading, INDENT);
        }

        if trimmed.starts_with('}') {
            let dedented = if leading.ends_with('\t') {
                leading.trim_end_matches('\t').to_string()
            } else if leading.ends_with(INDENT) {
                leading.trim_end_matches(INDENT).to_string()
            } else {
                String::new()
            };
            return format!("\n{}", dedented);
        }

        format!("\n{}", leading)
    }
}
