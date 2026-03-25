use crate::config::Config;
use crate::events::CustomEvent;
use crate::file_tree::FileTree;
use crate::file_watcher;
use crate::language::{HighlightSnapshot, HighlightingService, LanguageKind};
use crate::ui::{self, Action};
use lux_core::Buffer;
use notify::RecommendedWatcher;
use std::path::PathBuf;
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    egui_renderer: egui_wgpu::Renderer,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    event_proxy: EventLoopProxy<CustomEvent>,
    pub buffer: Buffer,
    pub workspace_path: Option<PathBuf>,
    file_tree: Option<FileTree>,
    _watcher: Option<RecommendedWatcher>,
    pub editor_config: Config,
    highlighting_service: HighlightingService,
}

impl State {
    pub async fn new(
        window: Arc<Window>,
        event_proxy: EventLoopProxy<CustomEvent>,
        initial_path: Option<PathBuf>,
    ) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
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
            .request_device(&wgpu::DeviceDescriptor::default())
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
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        egui_ctx.set_fonts(fonts);
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            egui_wgpu::RendererOptions::default(),
        );

        let mut state = Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            egui_renderer,
            egui_ctx,
            egui_state,
            event_proxy,
            buffer: Buffer::new(),
            workspace_path: None,
            file_tree: None,
            _watcher: None,
            editor_config: Config::load(),
            highlighting_service: HighlightingService::new(),
        };

        state.initialize_from_path(initial_path).await;
        state.refresh_language_intelligence();
        state
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.egui_state
            .on_window_event(&self.window, event)
            .consumed
    }

    pub fn on_file_change(&mut self) {
        if let Some(path) = &self.workspace_path {
            self.file_tree = Some(FileTree::new(path, self.event_proxy.clone()));
        }
    }

    pub fn open_folder(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        self.workspace_path = Some(path.clone());
        self.file_tree = Some(FileTree::new(&path, self.event_proxy.clone()));
        self.editor_config.add_recent(path.clone(), true);
        self._watcher = Self::start_watcher(&path, self.event_proxy.clone());
    }

    pub fn open_file(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        let proxy = self.event_proxy.clone();
        let path_clone = path.clone();
        tokio::spawn(async move {
            if let Ok(_buffer) = Buffer::from_file(&path_clone).await {
                proxy.send_event(CustomEvent::OpenFile(path_clone)).ok();
            }
        });
        self.editor_config.add_recent(path, false);
    }

    pub fn refresh_language_intelligence(&mut self) {
        let language = LanguageKind::from_path(self.buffer.path().map(|v| &**v));
        self.highlighting_service
            .request_parse(self.buffer.text().to_string(), language);
    }

    pub fn highlight_snapshot(&self) -> &HighlightSnapshot {
        self.highlighting_service.snapshot()
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.highlighting_service.update();
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

        let mut action = None;
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            action = ui::draw_ui(
                ctx,
                self.file_tree.as_ref(),
                self.workspace_path.as_ref(),
                &self.buffer,
                self.highlight_snapshot(),
                &self.editor_config,
                &self.event_proxy,
            );
        });

        if let Some(action) = action {
            match action {
                Action::OpenFile(path) => self.open_file(path),
                Action::OpenFolder(path) => self.open_folder(path),
            }
        }

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);
        let tris = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
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
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
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
            let mut render_pass = render_pass.forget_lifetime();
            self.egui_renderer
                .render(&mut render_pass, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    async fn initialize_from_path(&mut self, initial_path: Option<PathBuf>) {
        let Some(path) = initial_path else {
            return;
        };
        let path = path.canonicalize().unwrap_or(path);

        if path.is_dir() {
            self.workspace_path = Some(path.clone());
            self.file_tree = Some(FileTree::new(&path, self.event_proxy.clone()));
            self.editor_config.add_recent(path.clone(), true);
            self._watcher = Self::start_watcher(&path, self.event_proxy.clone());
            return;
        }

        if path.is_file()
            && let Ok(buffer) = Buffer::from_file(&path).await
        {
            self.buffer = buffer;
            self.window.set_title(&format!("lux - {}", path.display()));
            self.editor_config.add_recent(path, false);
        }
    }

    fn start_watcher(
        workspace_path: &PathBuf,
        event_proxy: EventLoopProxy<CustomEvent>,
    ) -> Option<RecommendedWatcher> {
        if let Ok((watcher, mut rx)) = file_watcher::watch(workspace_path) {
            tokio::spawn(async move {
                while let Some(result) = rx.recv().await {
                    match result {
                        Ok(_) => {
                            event_proxy.send_event(CustomEvent::FileChange).ok();
                        }
                        Err(error) => println!("watch error: {:?}", error),
                    }
                }
            });
            Some(watcher)
        } else {
            None
        }
    }
}
