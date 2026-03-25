use lux_core::Buffer;
use notify::RecommendedWatcher;
use std::sync::Arc;
use winit::{
    event::*,
    event_loop::{EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

mod file_tree;
use file_tree::FileTree;

mod file_watcher;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct RecentItem {
    path: std::path::PathBuf,
    is_dir: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Config {
    recent_items: Vec<RecentItem>,
}

impl Config {
    fn load() -> Self {
        let path = Self::path();
        if let Ok(data) = std::fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if let Ok(data) = serde_json::to_string(self) {
            std::fs::write(path, data).ok();
        }
    }

    fn path() -> std::path::PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("lux")
            .join("config.json")
    }

    fn add_recent(&mut self, path: std::path::PathBuf, is_dir: bool) {
        let item = RecentItem { path, is_dir };
        self.recent_items.retain(|i| i.path != item.path);
        self.recent_items.insert(0, item);
        if self.recent_items.len() > 10 {
            self.recent_items.truncate(10);
        }
        self.save();
    }
}

#[derive(Debug)]
enum CustomEvent {
    FileChange,
    OpenFile(std::path::PathBuf),
    Delete(std::path::PathBuf),
    Rename(std::path::PathBuf, std::path::PathBuf),
    NewFile(std::path::PathBuf),
    NewFolder(std::path::PathBuf),
}

struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    egui_renderer: egui_wgpu::Renderer,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    event_proxy: winit::event_loop::EventLoopProxy<CustomEvent>,
    buffer: Buffer,
    workspace_path: Option<std::path::PathBuf>,
    file_tree: Option<FileTree>,
    _watcher: Option<RecommendedWatcher>,
    editor_config: Config,
}

impl State {
    async fn new(
        window: Arc<Window>,
        event_loop: &EventLoop<CustomEvent>,
        initial_path: Option<std::path::PathBuf>,
    ) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(&device, config.format, None, 1);

        let mut buffer = Buffer::new();
        let mut workspace_path = None;
        let mut file_tree = None;
        let mut watcher = None;
        let event_proxy = event_loop.create_proxy();
        let mut editor_config = Config::load();

        if let Some(path) = initial_path {
            let path = path.canonicalize().unwrap_or(path);

            if path.is_dir() {
                workspace_path = Some(path.clone());
                file_tree = Some(FileTree::new(&path, event_proxy.clone()));
                editor_config.add_recent(path.clone(), true);

                // File watcher setup
                let proxy = event_proxy.clone();
                if let Ok((w, mut rx)) = file_watcher::watch(&path) {
                    watcher = Some(w);
                    tokio::spawn(async move {
                        while let Some(res) = rx.recv().await {
                            match res {
                                Ok(_) => {
                                    proxy.send_event(CustomEvent::FileChange).ok();
                                }
                                Err(e) => println!("watch error: {:?}", e),
                            }
                        }
                    });
                }
            } else if path.is_file() {
                if let Ok(b) = Buffer::from_file(&path).await {
                    buffer = b;
                    window.set_title(&format!("lux - {}", path.display()));
                    editor_config.add_recent(path, false);
                }
            }
        }

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            egui_renderer,
            egui_ctx,
            egui_state,
            event_proxy,
            buffer,
            workspace_path,
            file_tree,
            _watcher: watcher,
            editor_config,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        self.egui_state.on_window_event(&self.window, event).consumed
    }

    fn on_file_change(&mut self) {
        if let Some(path) = &self.workspace_path {
            self.file_tree = Some(FileTree::new(path, self.event_proxy.clone()));
        }
    }

    fn open_folder(&mut self, path: std::path::PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        self.workspace_path = Some(path.clone());
        self.file_tree = Some(FileTree::new(&path, self.event_proxy.clone()));
        self.editor_config.add_recent(path.clone(), true);

        // Reset watcher
        let proxy = self.event_proxy.clone();
        if let Ok((w, mut rx)) = file_watcher::watch(&path) {
            self._watcher = Some(w);
            tokio::spawn(async move {
                while let Some(res) = rx.recv().await {
                    match res {
                        Ok(_) => {
                            proxy.send_event(CustomEvent::FileChange).ok();
                        }
                        Err(e) => println!("watch error: {:?}", e),
                    }
                }
            });
        }
    }

    fn open_file(&mut self, path: std::path::PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        let proxy = self.event_proxy.clone();
        let path_clone = path.clone();
        tokio::spawn(async move {
            if let Ok(_b) = Buffer::from_file(&path_clone).await {
                proxy.send_event(CustomEvent::OpenFile(path_clone)).ok();
            }
        });
        self.editor_config.add_recent(path, false);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let raw_input = self.egui_state.take_egui_input(&self.window);
        
        enum Action {
            OpenFile(std::path::PathBuf),
            OpenFolder(std::path::PathBuf),
        }
        let mut action = None;

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            if let Some(file_tree) = &self.file_tree {
                egui::SidePanel::left("file_tree")
                    .resizable(true)
                    .default_width(200.0)
                    .width_range(100.0..=500.0)
                    .show(ctx, |ui| {
                        if let Some(path) = file_tree.show(ui) {
                            self.event_proxy.send_event(CustomEvent::OpenFile(path)).ok();
                        }
                    });
            }

            egui::CentralPanel::default().show(ctx, |ui| {
                if self.workspace_path.is_none() && self.buffer.path().is_none() {
                    // Welcome Page
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading(egui::RichText::new("Lux Editor").size(48.0).strong());
                        ui.add_space(20.0);
                        
                        ui.horizontal(|ui| {
                            ui.columns(2, |columns| {
                                columns[0].vertical_centered(|ui| {
                                    if ui.button(egui::RichText::new("Open File").size(20.0)).clicked() {
                                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                                            action = Some(Action::OpenFile(path));
                                        }
                                    }
                                });
                                columns[1].vertical_centered(|ui| {
                                    if ui.button(egui::RichText::new("Open Folder").size(20.0)).clicked() {
                                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                            action = Some(Action::OpenFolder(path));
                                        }
                                    }
                                });
                            });
                        });

                        ui.add_space(40.0);
                        ui.separator();
                        ui.add_space(20.0);
                        ui.heading("Recent Items");
                        ui.add_space(10.0);

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for item in &self.editor_config.recent_items {
                                let label = format!("{} ({})", item.path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown"), item.path.display());
                                if ui.selectable_label(false, label).clicked() {
                                    if item.is_dir {
                                        action = Some(Action::OpenFolder(item.path.clone()));
                                    } else {
                                        action = Some(Action::OpenFile(item.path.clone()));
                                    }
                                }
                            }
                        });
                    });
                } else {
                    // Editor View
                    ui.heading("Lux Editor");
                    
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let total_lines = self.buffer.len_lines();
                        let text_style = egui::TextStyle::Monospace;
                        let row_height = ui.text_style_height(&text_style);

                        ui.spacing_mut().item_spacing.y = 0.0;

                        egui::ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .show_rows(ui, row_height, total_lines, |ui, row_range| {
                                for i in row_range {
                                    if let Some(mut lines_iter) = self.buffer.line(i) {
                                        if let Some(line) = lines_iter.next() {
                                            ui.label(line.to_string());
                                        }
                                    }
                                }
                            });
                    });
                }
            });
        });

        if let Some(action) = action {
            match action {
                Action::OpenFile(path) => self.open_file(path),
                Action::OpenFolder(path) => self.open_folder(path),
            }
        }

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        let tris = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.size.width, self.size.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.egui_renderer.render(&mut render_pass, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub fn main() {
    env_logger::init();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();

    let event_loop = EventLoopBuilder::<CustomEvent>::with_user_event().build().unwrap();
    let window = Arc::new(WindowBuilder::new()
        .with_title("Lux Editor")
        .build(&event_loop)
        .unwrap());

    let initial_path = std::env::args().nth(1).map(std::path::PathBuf::from);
    let mut state = rt.block_on(State::new(Arc::clone(&window), &event_loop, initial_path));

    event_loop.run(move |event, elwt| {
        match event {
            Event::UserEvent(CustomEvent::FileChange) => {
                state.on_file_change();
            }
            Event::UserEvent(CustomEvent::OpenFile(path)) => {
                let path_clone = path.clone();
                if let Ok(b) = rt.block_on(Buffer::from_file(path)) {
                    state.buffer = b;
                    state.window().set_title(&format!("lux - {}", path_clone.display()));
                    state.editor_config.add_recent(path_clone, false);
                }
            }
            Event::UserEvent(CustomEvent::Delete(path)) => {
                rt.block_on(async {
                    if path.is_dir() {
                        tokio::fs::remove_dir_all(path).await.ok();
                    } else {
                        tokio::fs::remove_file(path).await.ok();
                    }
                });
            }
            Event::UserEvent(CustomEvent::NewFile(parent)) => {
                rt.block_on(async {
                    let path = parent.join("new_file.txt");
                    tokio::fs::File::create(path).await.ok();
                });
            }
            Event::UserEvent(CustomEvent::NewFolder(parent)) => {
                rt.block_on(async {
                    let path = parent.join("new_folder");
                    tokio::fs::create_dir(path).await.ok();
                });
            }
            Event::UserEvent(CustomEvent::Rename(old, new)) => {
                rt.block_on(async {
                    tokio::fs::rename(old, new).await.ok();
                });
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::KeyboardInput { event: key_event, .. } if key_event.state == ElementState::Pressed => {
                            if let winit::keyboard::Key::Character(to_insert) = &key_event.logical_key {
                                if !to_insert.starts_with(|c: char| c.is_ascii_control()) {
                                    // This is a simplistic way to handle input.
                                    // A real implementation would need to manage cursor position.
                                    let char_idx = state.buffer.text().len_chars();
                                    state.buffer.insert(char_idx, to_insert);
                                }
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
                        } => elwt.exit(),
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            match state.render() {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                                Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::AboutToWait => {
                state.window().request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}
