use crate::{
    converter::Converter,
    pipeline::Pipeline,
    vertex::{INDICES, VERTICES, Vertex},
};
use crossbeam_channel::Receiver;
use ffmpeg_next::util::frame::Video;
use tracing::{debug, info};
use wgpu::util::DeviceExt;
use winit::{
    event::{KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{Fullscreen, Window},
};

/// state of rendering engine
pub struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: &'a Window,
    vertex_buffer: wgpu::Buffer,
    frame_buffer: Receiver<Video>,
    is_fullscreen: bool,
    current_aspect_ratio: f32,
    converter: Converter,

    render_pipeline: Pipeline,
    display_bind_group: Option<wgpu::BindGroup>,
    display_bind_group_layout: wgpu::BindGroupLayout,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    pub video_width: u32,
    pub video_height: u32,
}

impl<'a> State<'a> {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &'a Window, frame_buffer: Receiver<Video>) -> State<'a> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::TEXTURE_FORMAT_16BIT_NORM,
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web, we'll have to disable some.
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();
        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
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

        let converter = Converter::new(&device);

        // vertex / index buffers
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let display_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // Texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    // Sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("display_bind_group_layout"),
            });

        // Render pipeline
        let render_pipeline = Pipeline::new(&device, &config, &display_bind_group_layout);

        // index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        let display_bind_group = None;
        // (‼️) configure the surface once up‑front
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            vertex_buffer,
            is_fullscreen: false,
            current_aspect_ratio: 1.0,
            frame_buffer,
            converter,

            render_pipeline,
            display_bind_group,
            display_bind_group_layout,
            index_buffer,
            num_indices,

            video_width: 0,
            video_height: 0,
        }
    }

    pub fn resize_window(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        info!("resize({}x{})", new_size.width, new_size.height);
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Update vertex buffer for new window aspect ratio
            self.update_vertex_buffer_for_aspect_ratio();
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state: winit::event::ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                info!("Key pressed: {:?}", physical_key);
                match physical_key {
                    PhysicalKey::Code(KeyCode::F11) => {
                        info!("F11 key detected, toggling fullscreen");
                        self.toggle_fullscreen();
                        true
                    }
                    PhysicalKey::Code(KeyCode::Escape) => {
                        info!("Escape key detected");
                        if self.is_fullscreen {
                            info!("Exiting fullscreen via Escape");
                            self.exit_fullscreen();
                            true
                        } else {
                            false
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyF) => {
                        info!("F key detected, toggling fullscreen");
                        self.toggle_fullscreen();
                        true
                    }
                    PhysicalKey::Code(KeyCode::Space) => {
                        info!("Space key detected, toggling fullscreen");
                        self.toggle_fullscreen();
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn toggle_fullscreen(&mut self) {
        self.is_fullscreen = !self.is_fullscreen;

        if self.is_fullscreen {
            // Enter fullscreen mode
            self.window
                .set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            // Exit fullscreen mode
            self.window.set_fullscreen(None);
        }

        info!("Fullscreen toggled: {}", self.is_fullscreen);
    }

    pub fn exit_fullscreen(&mut self) {
        if self.is_fullscreen {
            self.is_fullscreen = false;
            self.window.set_fullscreen(None);
            info!("Exited fullscreen mode");
        }
    }

    /// Update vertex buffer to maintain video aspect ratio
    fn update_vertex_buffer_for_aspect_ratio(&mut self) {
        if self.current_aspect_ratio <= 0.0 {
            panic!("Invalid aspect ratio"); // this check looks like bullshit so let's see if it is
            // ever helpful
        }

        let window_aspect_ratio = self.size.width as f32 / self.size.height as f32;

        let (scale_x, scale_y) = if self.current_aspect_ratio > window_aspect_ratio {
            // Video is wider than window - fit to width, letterbox top/bottom
            (1.0, window_aspect_ratio / self.current_aspect_ratio)
        } else {
            // Video is taller than window - fit to height, pillarbox left/right
            (self.current_aspect_ratio / window_aspect_ratio, 1.0)
        };

        // Create new vertices with aspect ratio correction
        let corrected_vertices = [
            Vertex {
                pos: [-scale_x, -scale_y],
                uv: [0.0, 1.0],
            },
            Vertex {
                pos: [scale_x, -scale_y],
                uv: [1.0, 1.0],
            },
            Vertex {
                pos: [scale_x, scale_y],
                uv: [1.0, 0.0],
            },
            Vertex {
                pos: [-scale_x, scale_y],
                uv: [0.0, 0.0],
            },
        ];

        // Update the vertex buffer
        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("aspect_ratio_vertex_buffer"),
                contents: bytemuck::cast_slice(&corrected_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        debug!(
            "Updated vertex buffer for aspect ratio: video={:.3}, window={:.3}, scale=({:.3}, {:.3})",
            self.current_aspect_ratio, window_aspect_ratio, scale_x, scale_y
        );
    }

    pub fn update_texture_with_new_frame(&mut self) {
        let frame = self.frame_buffer.recv().expect("failed to receive frame");
        debug!("{:?}", frame.format());

        let width = frame.width() as u32;
        let height = frame.height() as u32;

        if self.current_aspect_ratio <= 0.0
            || self.video_width != width
            || self.video_height != height
        {
            self.video_width = width;
            self.video_height = height;
            self.current_aspect_ratio = width as f32 / height as f32;
            self.update_vertex_buffer_for_aspect_ratio();
        }

        self.converter.process_frame(&self.device, &self.queue, &frame);
        self.setup_display_bind_group();

        debug!("Processed frame: {}x{} {:?}", width, height, frame.format());
    }

    fn setup_display_bind_group(&mut self) {
        if let Some(universal_texture) = self.converter.universal_texture() {
            let universal_view = universal_texture.create_view(&Default::default());

            let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            self.display_bind_group =
                Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &self.display_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&universal_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                    label: Some("display_bind_group"),
                }));

            debug!("Setup display bind group");
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&Default::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // render if we have a display bind group to group
            if let Some(display_bind_group) = &self.display_bind_group {
                rpass.set_pipeline(&self.render_pipeline.inner);
                rpass.set_bind_group(0, display_bind_group, &[]);
                rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                rpass.draw_indexed(0..self.num_indices, 0, 0..1);

                debug!("Rendered frame");
            } else {
                debug!("No display bind group - skipping render");
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }


}
