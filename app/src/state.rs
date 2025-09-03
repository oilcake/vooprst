use crate::vertex::{Vertex, INDICES, VERTICES};
use crossbeam_channel::Receiver;
use ffmpeg_next::util::frame::Video;
use wgpu::util::DeviceExt;
use winit::{
    event::{KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{Fullscreen, Window},
};
struct YUV {
    y: wgpu::Texture,
    u: wgpu::Texture,
    v: wgpu::Texture,
}

/// state of rendering engine
pub struct State<'a> {
    surface: wgpu::Surface<'a>,
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
    yuv_texture: YUV,
    frame_buffer: Receiver<Video>,
    texture_width: u32,
    texture_height: u32,
    is_fullscreen: bool,
    video_aspect_ratio: f32,
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

        // 1×1 текстуры для старта (R8Unorm)
        let y_size = wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let u_size = y_size;
        let v_size = y_size;

        let y_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: y_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texY"),
            view_formats: &[],
        });
        let u_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: u_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texU"),
            view_formats: &[],
        });
        let v_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: v_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texV"),
            view_formats: &[],
        });

        let yuv_texture = YUV {
            y: y_texture,
            u: u_texture,
            v: v_texture,
        };

        let y_view = yuv_texture.y.create_view(&Default::default());
        let u_view = yuv_texture.u.create_view(&Default::default());
        let v_view = yuv_texture.v.create_view(&Default::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // !!! Новый layout: 1 sampler + 3 текстуры
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: true }, // R8Unorm
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
            label: Some("yuv_bind_group_layout"),
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
                    resource: wgpu::BindingResource::TextureView(&y_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&u_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&v_view),
                },
            ],
            label: Some("yuv_bind_group"),
        });

        // shader & pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("yuv to rgba scaler"),
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
            yuv_texture,
            texture_width: 1,
            texture_height: 1,
            is_fullscreen: false,
            video_aspect_ratio: 1.0,
            frame_buffer,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn recreate_yuv_textures(&mut self, width: u32, height: u32) {
        // Y: WxH, U: W/2 x H/2, V: W/2 x H/2
        let y_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let uv_size = wgpu::Extent3d {
            width: width / 2,
            height: height / 2,
            depth_or_array_layers: 1,
        };

        self.yuv_texture.y = self.device.create_texture(&wgpu::TextureDescriptor {
            size: y_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texY"),
            view_formats: &[],
        });
        self.yuv_texture.u = self.device.create_texture(&wgpu::TextureDescriptor {
            size: uv_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texU"),
            view_formats: &[],
        });
        self.yuv_texture.v = self.device.create_texture(&wgpu::TextureDescriptor {
            size: uv_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("texV"),
            view_formats: &[],
        });

        // пересоздать views + bind group
        let y_view = self.yuv_texture.y.create_view(&Default::default());
        let u_view = self.yuv_texture.u.create_view(&Default::default());
        let v_view = self.yuv_texture.v.create_view(&Default::default());

        // (пере)создавать layout не обязательно — он такой же; но bind group пересобираем
        // sampler лучше сохранить в поле, если хочешь — у меня он локальный в new()
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // layout тот же, что в new(), переиспользуй его, если сохранил; здесь — коротко:
        let tex_layout = self.render_pipeline.get_bind_group_layout(0); // можно так, раз layout в pipeline[0]

        self.texture_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&y_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&u_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&v_view),
                },
            ],
            label: Some("yuv_bind_group"),
        });

        self.texture_width = width;
        self.texture_height = height;
        self.video_aspect_ratio = width as f32 / height as f32;
        self.update_vertex_buffer_for_aspect_ratio();
    }

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
                event:
                    KeyEvent {
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
            self.window
                .set_fullscreen(Some(Fullscreen::Borderless(None)));
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

        log::debug!("Updated vertex buffer for aspect ratio: video={:.3}, window={:.3}, scale=({:.3}, {:.3})", 
                   self.video_aspect_ratio, window_aspect_ratio, scale_x, scale_y);
    }

    pub fn update_texture_with_new_frame(&mut self) {
        let frame = self.frame_buffer.recv().expect("failed to receive frame");
        let width = frame.width() as u32;
        let height = frame.height() as u32;
        // Пересоздать текстуры, если размер изменился
        if self.texture_width != width || self.texture_height != height {
            self.recreate_yuv_textures(width, height);
        }

        // Проверим формат
        let fmt = frame.format();
        match fmt {
            ffmpeg_next::format::Pixel::YUV420P => self.update_yuv_textures_with_new_yuv420p_frame(&frame),
            _ => {
                // На первом шаге поддерживаем только 420p.
                // Тут можно: (а) вернуть рано, (б) залогировать, (в) временно игнорировать.
                log::error!(
                    "Unsupported pixel format: {:?}. Expected YUV420P (8-bit).",
                    fmt
                );
                return;
            }
        }
    }
    fn update_yuv_textures_with_new_yuv420p_frame(&mut self, frame: &Video) {
        let width = frame.width() as u32;
        let height = frame.height() as u32;
        // Плоскости
        let y_data = frame.data(0);
        let u_data = frame.data(1);
        let v_data = frame.data(2);

        let y_stride = frame.stride(0) as u32; // байт на строку
        let u_stride = frame.stride(1) as u32;
        let v_stride = frame.stride(2) as u32;

        // ожидаемые длины строки
        let y_row = width; // R8Unorm => 1 байт на пиксель
        let u_row = width / 2;
        let v_row = width / 2;

        // Y (WxH), U/V (W/2 x H/2)
        copy_plane(
            &self.queue,
            &self.yuv_texture.y,
            y_data,
            y_stride,
            width,
            height,
            y_row,
        );
        copy_plane(
            &self.queue,
            &self.yuv_texture.u,
            u_data,
            u_stride,
            width / 2,
            height / 2,
            u_row,
        );
        copy_plane(
            &self.queue,
            &self.yuv_texture.v,
            v_data,
            v_stride,
            width / 2,
            height / 2,
            v_row,
        );
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

// helper: допаковать, если stride != row
fn copy_plane(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    src: &[u8],
    stride: u32,
    w: u32,
    h: u32,
    row_bytes: u32,
) {
    if stride == row_bytes {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            src,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    } else {
        let mut packed = Vec::with_capacity((row_bytes * h) as usize);
        for y in 0..h {
            let from = (y * stride) as usize;
            let to = from + row_bytes as usize;
            packed.extend_from_slice(&src[from..to]);
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &packed,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }
}
