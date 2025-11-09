use crate::{
    compute_converter::{ComputeConverter, ConverterParams},
    pipeline::Pipeline,
    vertex::{INDICES, VERTICES, Vertex},
};
use crossbeam_channel::Receiver;
use ffmpeg_next::util::frame::Video;
use tracing::{debug, error, info};
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
    // texture_bind_group: wgpu::BindGroup,
    // render_pipelines: HashMap<usize, Pipeline>,
    // current_pipeline: usize,
    vertex_buffer: wgpu::Buffer,
    // index_buffer: wgpu::Buffer,
    // num_indices: u32,
    // yuv_texture: YUV,
    frame_buffer: Receiver<Video>,
    is_fullscreen: bool,
    video_aspect_ratio: f32,
    compute_converter: ComputeConverter,
    converter_bind_group: Option<wgpu::BindGroup>,
    universal_texture: Option<wgpu::Texture>, // Текстура в RGBA16Float
    params_buffer: wgpu::Buffer,

    render_pipeline: Pipeline,
    display_bind_group: Option<wgpu::BindGroup>,
    display_bind_group_layout: wgpu::BindGroupLayout,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    video_width: u32, // Добавляем эти два поля
    video_height: u32,
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

        // 1×1 default texture to init with (R8Unorm)
        // let y_size = wgpu::Extent3d {
        //     width: 1,
        //     height: 1,
        //     depth_or_array_layers: 1,
        // };
        // let u_size = y_size;
        // let v_size = y_size;
        //
        // let y_texture = device.create_texture(&wgpu::TextureDescriptor {
        //     size: y_size,
        //     mip_level_count: 1,
        //     sample_count: 1,
        //     dimension: wgpu::TextureDimension::D2,
        //     format: wgpu::TextureFormat::R8Unorm,
        //     usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        //     label: Some("texY"),
        //     view_formats: &[],
        // });
        // let u_texture = device.create_texture(&wgpu::TextureDescriptor {
        //     size: u_size,
        //     mip_level_count: 1,
        //     sample_count: 1,
        //     dimension: wgpu::TextureDimension::D2,
        //     format: wgpu::TextureFormat::R8Unorm,
        //     usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        //     label: Some("texU"),
        //     view_formats: &[],
        // });
        // let v_texture = device.create_texture(&wgpu::TextureDescriptor {
        //     size: v_size,
        //     mip_level_count: 1,
        //     sample_count: 1,
        //     dimension: wgpu::TextureDimension::D2,
        //     format: wgpu::TextureFormat::R8Unorm,
        //     usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        //     label: Some("texV"),
        //     view_formats: &[],
        // });
        //
        // let yuv_texture = YUV {
        //     y: Plane {
        //         texture: y_texture,
        //         size: y_size,
        //     },
        //     u: Plane {
        //         texture: u_texture,
        //         size: u_size,
        //     },
        //     v: Plane {
        //         texture: v_texture,
        //         size: v_size,
        //     },
        //     format: wgpu::TextureFormat::R8Unorm,
        // };
        //
        // let y_view = yuv_texture.y.texture.create_view(&Default::default());
        // let u_view = yuv_texture.u.texture.create_view(&Default::default());
        // let v_view = yuv_texture.v.texture.create_view(&Default::default());
        //
        // let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        //     address_mode_u: wgpu::AddressMode::ClampToEdge,
        //     address_mode_v: wgpu::AddressMode::ClampToEdge,
        //     mag_filter: wgpu::FilterMode::Linear,
        //     min_filter: wgpu::FilterMode::Linear,
        //     ..Default::default()
        // });
        //
        // // !!! Новый layout: 1 sampler + 3 текстуры
        // let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //     entries: &[
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 0,
        //             visibility: wgpu::ShaderStages::FRAGMENT,
        //             ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        //             count: None,
        //         },
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 1,
        //             visibility: wgpu::ShaderStages::FRAGMENT,
        //             ty: wgpu::BindingType::Texture {
        //                 multisampled: false,
        //                 view_dimension: wgpu::TextureViewDimension::D2,
        //                 sample_type: wgpu::TextureSampleType::Float { filterable: true }, // R8Unorm
        //             },
        //             count: None,
        //         },
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 2,
        //             visibility: wgpu::ShaderStages::FRAGMENT,
        //             ty: wgpu::BindingType::Texture {
        //                 multisampled: false,
        //                 view_dimension: wgpu::TextureViewDimension::D2,
        //                 sample_type: wgpu::TextureSampleType::Float { filterable: true },
        //             },
        //             count: None,
        //         },
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 3,
        //             visibility: wgpu::ShaderStages::FRAGMENT,
        //             ty: wgpu::BindingType::Texture {
        //                 multisampled: false,
        //                 view_dimension: wgpu::TextureViewDimension::D2,
        //                 sample_type: wgpu::TextureSampleType::Float { filterable: true },
        //             },
        //             count: None,
        //         },
        //     ],
        //     label: Some("yuv_bind_group_layout"),
        // });
        // let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &tex_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::Sampler(&sampler),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::TextureView(&y_view),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 2,
        //             resource: wgpu::BindingResource::TextureView(&u_view),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 3,
        //             resource: wgpu::BindingResource::TextureView(&v_view),
        //         },
        //     ],
        //     label: Some("yuv_bind_group"),
        // });
        //
        // Создаем compute конвертер
        let compute_converter = ComputeConverter::new(&device);

        // Создаем буфер для параметров
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("converter_params_buffer"),
            size: std::mem::size_of::<ConverterParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Универсальная текстура будет создаваться при получении первого кадра
        let universal_texture = None;
        let converter_bind_group = None;
        //
        // let pixel_format = ffmpeg_next::format::Pixel::YUV420P;
        //
        // let render_pipeline =
        //     crate::pipeline::Pipeline::new(&device, &config, &tex_layout);
        //
        // let mut render_pipelines = HashMap::new();
        // render_pipelines.insert(pixel_format as usize, render_pipeline);
        //
        // // vertex / index buffers
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("index_buffer"),
        //     contents: bytemuck::cast_slice(INDICES),
        //     usage: wgpu::BufferUsages::INDEX,
        // });
        //
        let display_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    // Текстура для отображения
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
                    // Сэмплер
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("display_bind_group_layout"),
            });

        // Создаем рендер пайплайн
        let render_pipeline = Pipeline::new(&device, &config, &display_bind_group_layout);

        // Создаем index buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        let display_bind_group = None;
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
            // texture_bind_group,
            // render_pipelines,
            // current_pipeline: pixel_format as usize,
            vertex_buffer,
            // index_buffer,
            // num_indices: INDICES.len() as u32,
            // yuv_texture,
            is_fullscreen: false,
            video_aspect_ratio: 1.0,
            frame_buffer,
            compute_converter,
            converter_bind_group,
            universal_texture,
            params_buffer,

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

        debug!(
            "Updated vertex buffer for aspect ratio: video={:.3}, window={:.3}, scale=({:.3}, {:.3})",
            self.video_aspect_ratio, window_aspect_ratio, scale_x, scale_y
        );
    }

    pub fn update_texture_with_new_frame(&mut self) {
        let frame = self.frame_buffer.recv().expect("failed to receive frame");
        debug!("{:?}", frame.format());
        // Пока работаем только с YUV420P
        if frame.format() != ffmpeg_next::format::Pixel::YUV420P {
            debug!("Skipping non-YUV420P frame");
            return;
        }

        let width = frame.width() as u32;
        let height = frame.height() as u32;
        
        // ВЫЗЫВАЕМ ОБНОВЛЕНИЕ ASPECT RATIO ПЕРЕД ВСЕМ ОСТАЛЬНЫМ
        if self.video_aspect_ratio <= 0.0 || self.video_width != width || self.video_height != height {
            self.video_width = width;
            self.video_height = height;
            self.video_aspect_ratio = width as f32 / height as f32;
            self.update_vertex_buffer_for_aspect_ratio(); // ← ДОБАВИТЬ ЭТУ СТРОЧКУ
        }

        // Создаем текстуру для вывода если ее нет
        if self.universal_texture.is_none() {
            self.recreate_universal_texture(width, height);
        }

        // Создаем входную текстуру для Y-plane
        let input_texture = self.create_y_plane_texture(&frame);

        // Загружаем Y-plane данные
        self.upload_y_plane_data(&frame, &input_texture);

        // Настраиваем bind group
        self.setup_converter_bind_group(&input_texture);

        // Обновляем параметры
        self.update_converter_params(width, height);

        // Запускаем compute шейдер
        self.run_converter(width, height);

        debug!("Successfully processed YUV420P frame: {}x{}", width, height);
    }

    fn recreate_universal_texture(&mut self, width: u32, height: u32) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("universal_texture"),
            view_formats: &[],
        });

        self.universal_texture = Some(texture);
        debug!("Created universal texture: {}x{}", width, height);
    }

    fn create_y_plane_texture(&self, frame: &Video) -> wgpu::Texture {
        let width = frame.width() as u32;
        let height = frame.height() as u32;

        self.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("y_plane_texture"),
            view_formats: &[],
        })
    }

    fn upload_y_plane_data(&self, frame: &Video, texture: &wgpu::Texture) {
        let width = frame.width() as u32;
        let height = frame.height() as u32;
        let y_data = frame.data(0);
        let stride = frame.stride(0) as u32;

        let expected_row_bytes = width; // R8Unorm = 1 байт на пиксель

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            y_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(expected_row_bytes),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        debug!("Uploaded Y-plane data: {} bytes", y_data.len());
    }

    fn setup_converter_bind_group(&mut self, input_texture: &wgpu::Texture) {
        let input_view = input_texture.create_view(&Default::default());
        let universal_view = self
            .universal_texture
            .as_ref()
            .unwrap()
            .create_view(&Default::default());

        self.converter_bind_group =
            Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.compute_converter.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&input_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&universal_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.params_buffer,
                            offset: 0,
                            size: None,
                        }),
                    },
                ],
                label: Some("converter_bind_group"),
            }));

        // Теперь создаем и display_bind_group
        self.setup_display_bind_group();

        debug!("Setup converter and display bind groups");
    }

    fn setup_display_bind_group(&mut self) {
        if let Some(universal_texture) = &self.universal_texture {
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

    // Обновляем метод render чтобы реально рендерить
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

            // Рендерим только если есть что показывать
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

    fn update_converter_params(&mut self, width: u32, height: u32) {
        let params = ConverterParams {
            format: 0, // YUV420P
            width,
            height,
        };

        self.queue
            .write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

        debug!("Updated converter params: {}x{}", width, height);
    }

    fn run_converter(&mut self, width: u32, height: u32) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut compute_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.compute_converter.pipeline);
            compute_pass.set_bind_group(0, self.converter_bind_group.as_ref().unwrap(), &[]);

            let workgroups_x = (width + 7) / 8;
            let workgroups_y = (height + 7) / 8;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        self.queue.submit(Some(encoder.finish()));
        debug!(
            "Dispatched compute shader: {}x{} workgroups",
            (width + 7) / 8,
            (height + 7) / 8
        );
    }
}
