use crate::vertex::{Vertex, INDICES, VERTICES};
use ffmpeg_next as ffmpeg;
use wgpu::util::DeviceExt;
use winit::{event::{WindowEvent, KeyEvent}, window::{Window, Fullscreen}, keyboard::{KeyCode, PhysicalKey}};

/// state of rendering engine
pub struct State<'a> {
    surface: wgpu::Surface<'a>,
    pub surface_configured: bool,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    window: &'a Window,
    texture_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    diffuse_texture: wgpu::Texture,
    texture_width: u32,
    texture_height: u32,
    is_fullscreen: bool,
    video_aspect_ratio: f32,
}

impl<'a> State<'a> {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &'a Window) -> State<'a> {
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
        dbg!(&adapter);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web, we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
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

        // Create a placeholder texture that will be updated with video frames
        // We'll start with a 1x1 texture and resize it when we get the first frame
        let texture_size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("diffuse_texture"),
            view_formats: &[],
        });

        // Initialize with a single black pixel
        let black_pixel = [0u8, 0u8, 0u8, 255u8];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &black_pixel,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            texture_size,
        );

        // view + sampler
        let diffuse_view = diffuse_texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // bind‑group layout + instance
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&diffuse_view),
                },
            ],
            label: Some("texture_bind_group"),
        });

        // shader & pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("texture_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&tex_layout],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("texture_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"), // ← now Option<&str>
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(), // ← new field
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"), // ← now Option<&str>
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(), // ← new field
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None, // ← new field
        });

        // vertex / index buffers
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        // (‼️) configure the surface once up‑front
        surface.configure(&device, &config);

        // final return
        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            texture_bind_group,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: INDICES.len() as u32,
            diffuse_texture,
            texture_width: 1,
            texture_height: 1,
            is_fullscreen: false,
            video_aspect_ratio: 1.0,
            surface_configured: false,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn recreate_texture(&mut self, width: u32, height: u32) {
        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        self.diffuse_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("diffuse_texture"),
            view_formats: &[],
        });

        // Recreate the texture view and bind group
        let diffuse_view = self.diffuse_texture.create_view(&Default::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let tex_layout = self
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        self.texture_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&diffuse_view),
                },
            ],
            label: Some("texture_bind_group"),
        });

        self.texture_width = width;
        self.texture_height = height;
        self.video_aspect_ratio = width as f32 / height as f32;
        
        // Update vertex buffer with new aspect ratio
        self.update_vertex_buffer_for_aspect_ratio();
    }

    // impl State
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        log::info!("resize({}x{})", new_size.width, new_size.height);
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
                event: KeyEvent {
                    physical_key,
                    state: winit::event::ElementState::Pressed,
                    ..
                },
                ..
            } => {
                log::info!("Key pressed: {:?}", physical_key);
                match physical_key {
                    PhysicalKey::Code(KeyCode::F11) => {
                        log::info!("F11 key detected, toggling fullscreen");
                        self.toggle_fullscreen();
                        true
                    }
                    PhysicalKey::Code(KeyCode::Escape) => {
                        log::info!("Escape key detected");
                        if self.is_fullscreen {
                            log::info!("Exiting fullscreen via Escape");
                            self.exit_fullscreen();
                            true
                        } else {
                            false
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyF) => {
                        log::info!("F key detected, toggling fullscreen");
                        self.toggle_fullscreen();
                        true
                    }
                    PhysicalKey::Code(KeyCode::Space) => {
                        log::info!("Space key detected, toggling fullscreen");
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
            self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            // Exit fullscreen mode
            self.window.set_fullscreen(None);
        }
        
        log::info!("Fullscreen toggled: {}", self.is_fullscreen);
    }

    pub fn exit_fullscreen(&mut self) {
        if self.is_fullscreen {
            self.is_fullscreen = false;
            self.window.set_fullscreen(None);
            log::info!("Exited fullscreen mode");
        }
    }

    /// Update vertex buffer to maintain video aspect ratio
    fn update_vertex_buffer_for_aspect_ratio(&mut self) {
        if self.video_aspect_ratio <= 0.0 {
            return; // Skip if we don't have valid video dimensions yet
        }

        let window_aspect_ratio = self.size.width as f32 / self.size.height as f32;
        
        let (scale_x, scale_y) = if self.video_aspect_ratio > window_aspect_ratio {
            // Video is wider than window - fit to width, letterbox top/bottom
            (1.0, window_aspect_ratio / self.video_aspect_ratio)
        } else {
            // Video is taller than window - fit to height, pillarbox left/right
            (self.video_aspect_ratio / window_aspect_ratio, 1.0)
        };

        // Create new vertices with aspect ratio correction
        let corrected_vertices = [
            Vertex { pos: [-scale_x, -scale_y], uv: [0.0, 1.0] },
            Vertex { pos: [ scale_x, -scale_y], uv: [1.0, 1.0] },
            Vertex { pos: [ scale_x,  scale_y], uv: [1.0, 0.0] },
            Vertex { pos: [-scale_x,  scale_y], uv: [0.0, 0.0] },
        ];

        // Update the vertex buffer
        self.vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("aspect_ratio_vertex_buffer"),
            contents: bytemuck::cast_slice(&corrected_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        log::debug!("Updated vertex buffer for aspect ratio: video={:.3}, window={:.3}, scale=({:.3}, {:.3})", 
                   self.video_aspect_ratio, window_aspect_ratio, scale_x, scale_y);
    }

    pub fn update(&mut self) {
        // todo!()
    }

    pub fn update_texture_with_frame(&mut self, frame: &ffmpeg::util::frame::Video) {
        let width = frame.width() as u32;
        let height = frame.height() as u32;
        let data = frame.data(0);
        let stride = frame.stride(0) as u32;

        // Check if we need to recreate the texture with new dimensions
        if self.texture_width != width || self.texture_height != height {
            self.recreate_texture(width, height);
        }

        // Calculate the actual row size (width * 4 bytes per pixel for RGBA)
        let row_size = width * 4;

        // If stride equals row size, we can copy directly
        if stride == row_size {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.diffuse_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_size),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        } else {
            // If stride is different, we need to copy row by row
            let mut packed_data = Vec::with_capacity((width * height * 4) as usize);
            for y in 0..height {
                let row_start = (y * stride) as usize;
                let row_end = row_start + row_size as usize;
                packed_data.extend_from_slice(&data[row_start..row_end]);
            }

            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.diffuse_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &packed_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_size),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&Default::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
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

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &self.texture_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}
