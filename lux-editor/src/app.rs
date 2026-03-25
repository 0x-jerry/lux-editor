use crate::events::CustomEvent;
use crate::state::State;
use lux_core::Buffer;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

pub struct App {
    rt: tokio::runtime::Runtime,
    proxy: EventLoopProxy<CustomEvent>,
    state: Option<State>,
    initial_path: Option<std::path::PathBuf>,
}

impl App {
    pub fn new(proxy: EventLoopProxy<CustomEvent>) -> Self {
        Self {
            rt: tokio::runtime::Runtime::new().unwrap(),
            proxy,
            state: None,
            initial_path: std::env::args().nth(1).map(std::path::PathBuf::from),
        }
    }

    fn handle_user_event(&mut self, event: CustomEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            CustomEvent::FileChange => state.on_file_change(),
            CustomEvent::ConfigChange => state.on_config_change(),
            CustomEvent::OpenFile(path) => {
                let title_path = path.clone();
                if let Ok(buffer) = self.rt.block_on(Buffer::from_file(path)) {
                    state.buffer = buffer;
                    state
                        .window()
                        .set_title(&format!("lux - {}", title_path.display()));
                    state.editor_config.add_recent(title_path, false);
                    state.refresh_language_intelligence();
                }
            }
            CustomEvent::Delete(path) => {
                self.rt.block_on(async {
                    if path.is_dir() {
                        tokio::fs::remove_dir_all(path).await.ok();
                    } else {
                        tokio::fs::remove_file(path).await.ok();
                    }
                });
            }
            CustomEvent::Rename(old, new) => {
                self.rt.block_on(async {
                    tokio::fs::rename(old, new).await.ok();
                });
            }
            CustomEvent::NewFile(parent) => {
                self.rt.block_on(async {
                    tokio::fs::File::create(parent.join("new_file.txt"))
                        .await
                        .ok();
                });
            }
            CustomEvent::NewFolder(parent) => {
                self.rt.block_on(async {
                    tokio::fs::create_dir(parent.join("new_folder")).await.ok();
                });
            }
        }
    }

    fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if window_id != state.window().id() {
            return;
        }

        if state.input(&event) {
            return;
        }

        match event {
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } if key_event.state == ElementState::Pressed => {
                let mut changed = false;
                match &key_event.logical_key {
                    Key::Named(NamedKey::Enter) => {
                        let indentation = Self::indentation_for_newline(&state.buffer);
                        let char_idx = state.buffer.text().len_chars();
                        state.buffer.insert(char_idx, &indentation);
                        changed = true;
                    }
                    Key::Named(NamedKey::Backspace) => {
                        let char_idx = state.buffer.text().len_chars();
                        if char_idx > 0 {
                            state.buffer.remove(char_idx - 1..char_idx);
                            changed = true;
                        }
                    }
                    Key::Named(NamedKey::Tab) => {
                        let char_idx = state.buffer.text().len_chars();
                        state.buffer.insert(char_idx, "    ");
                        changed = true;
                    }
                    Key::Character(to_insert) => {
                        if !to_insert.starts_with(|c: char| c.is_ascii_control()) {
                            let char_idx = state.buffer.text().len_chars();
                            state.buffer.insert(char_idx, to_insert);
                            changed = true;
                        }
                    }
                    _ => {}
                }

                if changed {
                    state.refresh_language_intelligence();
                }
            }
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key: winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::Resized(physical_size) => state.resize(physical_size),
            WindowEvent::RedrawRequested => match state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                Err(error) => eprintln!("{:?}", error),
            },
            _ => {}
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

impl ApplicationHandler<CustomEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("Lux Editor"))
                .unwrap(),
        );
        let initial_path = self.initial_path.take();
        let state = self
            .rt
            .block_on(State::new(window, self.proxy.clone(), initial_path));
        self.state = Some(state);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: CustomEvent) {
        self.handle_user_event(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.handle_window_event(event_loop, window_id, event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window().request_redraw();
        }
    }
}
