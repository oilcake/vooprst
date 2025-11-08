
use crate::vertex::{INDICES, VERTICES, Vertex};
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

    // compute_converter: ComputeConverter,
    // converter_bind_group: Option<wgpu::BindGroup>,
    // universal_texture: Option<wgpu::Texture>, // Текстура в RGBA16Float
    // params_buffer: wgpu::Buffer,
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
        // let compute_converter = ComputeConverter::new(&device);
        
        // // Создаем буфер для параметров
        // let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        //     label: Some("converter_params_buffer"),
        //     size: std::mem::size_of::<ConverterParams>() as u64,
        //     usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        //     mapped_at_creation: false,
        // });
        //
        // // Универсальная текстура будет создаваться при получении первого кадра
        // let universal_texture = None;
        // let converter_bind_group = None;
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
        // // (‼️) configure the surface once up‑front
        // surface.configure(&device, &config);

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
            // compute_converter,
            // converter_bind_group,
            // universal_texture,
            // params_buffer,
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
        // let width = frame.width() as u32;
        // let height = frame.height() as u32;
        // let format = frame.format();
        //
        // // Создаем/обновляем универсальную текстуру если нужно
        // if self.universal_texture.is_none() ||
        //    self.universal_texture.as_ref().unwrap().width() != width ||
        //    self.universal_texture.as_ref().unwrap().height() != height {
        //     self.recreate_universal_texture(width, height);
        // }
        //
        // // Создаем входную текстуру для сырых данных
        // let input_texture = self.create_input_texture_for_frame(&frame);
        // let input_view = input_texture.create_view(&Default::default());
        //
        // // Настраиваем bind group для конвертера
        // self.setup_converter_bind_group(&input_view);
        //
        // // Загружаем сырые данные во входную текстуру
        // self.upload_frame_data(&frame, &input_texture);
        //
        // // Обновляем параметры конвертации
        // self.update_converter_params(&frame);
        //
        // // Запускаем compute шейдер конвертации
        // self.run_converter(width, height);
    }

    // fn create_input_texture_for_frame(&self, frame: &Video) -> wgpu::Texture {
    //     let (format, size) = match frame.format() {
    //         ffmpeg_next::format::Pixel::YUV420P => (
    //             wgpu::TextureFormat::R8Unorm,
    //             wgpu::Extent3d {
    //                 width: frame.width() as u32,
    //                 height: frame.height() as u32 * 3/2, // Y + UV planes
    //                 depth_or_array_layers: 1,
    //             }
    //         ),
    //         ffmpeg_next::format::Pixel::YUV422P10LE => (
    //             wgpu::TextureFormat::R16Unorm,
    //             wgpu::Extent3d {
    //                 width: frame.width() as u32,
    //                 height: frame.height() as u32 * 2, // Y + UV planes
    //                 depth_or_array_layers: 1,
    //             }
    //         ),
    //         // Добавьте другие форматы по необходимости
    //         _ => panic!("Unsupported format: {:?}", frame.format()),
    //     };
    //
    //     self.device.create_texture(&wgpu::TextureDescriptor {
    //         size,
    //         mip_level_count: 1,
    //         sample_count: 1,
    //         dimension: wgpu::TextureDimension::D2,
    //         format,
    //         usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    //         label: Some("input_texture"),
    //         view_formats: &[],
    //     })
    // }

    // fn update_converter_params(&mut self, frame: &Video) {
    //     let params = ConverterParams {
    //         format: self.map_pixel_format_to_index(frame.format()),
    //         width: frame.width() as u32,
    //         height: frame.height() as u32,
    //         stride_y: frame.stride(0) as u32,
    //         stride_u: frame.stride(1) as u32,
    //         stride_v: frame.stride(2) as u32,
    //         bit_depth: self.get_bit_depth(frame.format()),
    //         color_matrix: 0, // BT709 по умолчанию
    //     };
    //
    //     self.queue.write_buffer(
    //         &self.params_buffer,
    //         0,
    //         bytemuck::cast_slice(&[params]),
    //     );
    // }

    // fn run_converter(&mut self, width: u32, height: u32) {
    //     let mut encoder = self.device.create_command_encoder(
    //         &wgpu::CommandEncoderDescriptor::default()
    //     );
    //
    //     {
    //         let mut compute_pass = encoder.begin_compute_pass(
    //             &wgpu::ComputePassDescriptor::default()
    //         );
    //         compute_pass.set_pipeline(&self.compute_converter.pipeline);
    //         compute_pass.set_bind_group(0, self.converter_bind_group.as_ref().unwrap(), &[]);
    //
    //         let workgroups_x = (width + 7) / 8;
    //         let workgroups_y = (height + 7) / 8;
    //         compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    //     }
    //
    //     self.queue.submit(Some(encoder.finish()));
    // }

    // fn map_pixel_format_to_index(&self, format: ffmpeg_next::format::Pixel) -> u32 {
    //     match format {
    //         ffmpeg_next::format::Pixel::YUV420P => 0,
    //         ffmpeg_next::format::Pixel::YUV422P => 1,
    //         ffmpeg_next::format::Pixel::NV12 => 2,
    //         ffmpeg_next::format::Pixel::RGB24 => 3,
    //         ffmpeg_next::format::Pixel::YUV422P10LE => 4,
    //         // Добавьте другие форматы
    //         _ => 0,
    //     }
    // }

    // fn get_bit_depth(&self, format: ffmpeg_next::format::Pixel) -> u32 {
    //     match format {
    //         ffmpeg_next::format::Pixel::YUV420P => 8,
    //         ffmpeg_next::format::Pixel::YUV422P10LE => 10,
    //         ffmpeg_next::format::Pixel::YUV422P12LE => 12,
    //         _ => 8,
    //     }
    // }

    // fn update_yuv_textures_with_new_yuv422p10le_frame(&mut self, frame: &Video) {
    //     let width = frame.width() as u32;
    //     let height = frame.height() as u32;
    //
    //     // плоскости (10-бит, реально 16-бит в памяти)
    //     let y_data = frame.data(0);
    //     let u_data = frame.data(1);
    //     let v_data = frame.data(2);
    //
    //     let y_stride = frame.stride(0) as u32; // байт на строку
    //     debug!("Y stride with 422: {}", y_stride);
    //     let u_stride = frame.stride(1) as u32;
    //     debug!("U stride with 422: {}", u_stride);
    //     let v_stride = frame.stride(2) as u32;
    //     debug!("V stride with 422: {}", v_stride);
    //
    //     // ожидаемые длины строки в байтах:
    //     // R16Unorm = 2 байта на выборку
    //     let y_row = width * 2;
    //     let u_row = (width / 2) * 2; // U texture is width/2, R16Unorm = 2 bytes per pixel
    //     let v_row = (width / 2) * 2; // V texture is width/2, R16Unorm = 2 bytes per pixel
    //
    //     let y_converted = convert_10bit_to_16unorm(y_data, width, height, y_stride);
    //     let u_converted = convert_10bit_to_16unorm(u_data, width / 2, height, u_stride);
    //     let v_converted = convert_10bit_to_16unorm(v_data, width / 2, height, v_stride);
    //
    //     // Y: W × H
    //     copy_plane_16bit(
    //         &self.queue,
    //         &self.yuv_texture.y.texture,
    //         &y_converted,
    //         y_stride,
    //         width,
    //         height,
    //         y_row,
    //     );
    //
    //     // U: W/2 × H
    //     copy_plane_16bit(
    //         &self.queue,
    //         &self.yuv_texture.u.texture,
    //         &u_converted,
    //         u_stride,
    //         width / 2,
    //         height,
    //         u_row,
    //     );
    //
    // V: W/2 × H
    //     copy_plane_16bit(
    //         &self.queue,
    //         &self.yuv_texture.v.texture,
    //         &v_converted,
    //         v_stride,
    //         width / 2,
    //         height,
    //         v_row,
    //     );
    // }

    // fn update_yuv_textures_with_new_yuv420p_frame(&mut self, frame: &Video) {
    //     let width = frame.width() as u32;
    //     let height = frame.height() as u32;
    //     // Плоскости
    //     let y_data = frame.data(0);
    //     let u_data = frame.data(1);
    //     let v_data = frame.data(2);
    //
    //     let y_stride = frame.stride(0) as u32; // байт на строку
    //     let u_stride = frame.stride(1) as u32;
    //     let v_stride = frame.stride(2) as u32;
    //
    //     // ожидаемые длины строки
    //     let y_row = width; // R8Unorm => 1 байт на пиксель
    //     let u_row = width / 2;
    //     let v_row = width / 2;
    //
    //     // Y (WxH), U/V (W/2 x H/2)
    //     copy_plane_8bit(
    //         &self.queue,
    //         &self.yuv_texture.y.texture,
    //         y_data,
    //         y_stride,
    //         width,
    //         height,
    //         y_row,
    //     );
    //     copy_plane_8bit(
    //         &self.queue,
    //         &self.yuv_texture.u.texture,
    //         u_data,
    //         u_stride,
    //         width / 2,
    //         height / 2,
    //         u_row,
    //     );
    //     copy_plane_8bit(
    //         &self.queue,
    //         &self.yuv_texture.v.texture,
    //         v_data,
    //         v_stride,
    //         width / 2,
    //         height / 2,
    //         v_row,
    //     );
    // }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        //     let frame = self.surface.get_current_texture()?;
        //     let view = frame.texture.create_view(&Default::default());
        //
        //     let mut encoder = self
        //         .device
        //         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        //             label: Some("encoder"),
        //         });
        //
        //     {
        //         let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        //             label: Some("render_pass"),
        //             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        //                 view: &view,
        //                 resolve_target: None,
        //                 ops: wgpu::Operations {
        //                     load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        //                     store: wgpu::StoreOp::Store,
        //                 },
        //             })],
        //             depth_stencil_attachment: None,
        //             occlusion_query_set: None,
        //             timestamp_writes: None,
        //         });
        //
        //         let render_pipeline = &self.render_pipelines.get(&self.current_pipeline).unwrap();
        //         rpass.set_pipeline(&render_pipeline.inner);
        //         // rpass.set_bind_group(0, &self.texture_bind_group, &[]);
        //         rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        //         rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        //         rpass.draw_indexed(0..self.num_indices, 0, 0..1);
        //     }
        //
        //     self.queue.submit(std::iter::once(encoder.finish()));
        //     frame.present();
        debug!("pretend we are rendering");
        Ok(())
    }
    // pub fn recreate_universal_texture(&mut self, width: u32, height: u32) {
    //     let texture = self.device.create_texture(&wgpu::TextureDescriptor {
    //         size: wgpu::Extent3d {
    //             width,
    //             height,
    //             depth_or_array_layers: 1,
    //         },
    //         mip_level_count: 1,
    //         sample_count: 1,
    //         dimension: wgpu::TextureDimension::D2,
    //         format: wgpu::TextureFormat::Rgba16Float, // Универсальный формат
    //         usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
    //         label: Some("universal_texture"),
    //         view_formats: &[],
    //     });
    //
    //     self.universal_texture = Some(texture);
    // }

    //     fn setup_converter_bind_group(&mut self, input_texture_view: &wgpu::TextureView) {
    //         let universal_view = self.universal_texture
    //             .as_ref()
    //             .unwrap()
    //             .create_view(&Default::default());
    //
    //         self.converter_bind_group = Some(
    //             self.device.create_bind_group(&wgpu::BindGroupDescriptor {
    //                 layout: &self.compute_converter.bind_group_layout,
    //                 entries: &[
    //                     wgpu::BindGroupEntry {
    //                         binding: 0,
    //                         resource: wgpu::BindingResource::TextureView(input_texture_view),
    //                     },
    //                     wgpu::BindGroupEntry {
    //                         binding: 1,
    //                         resource: wgpu::BindingResource::TextureView(&universal_view),
    //                     },
    //                     wgpu::BindGroupEntry {
    //                         binding: 2,
    //                         resource: wgpu::BindingResource::Buffer(
    //                             wgpu::BufferBinding {
    //                                 buffer: &self.params_buffer,
    //                                 offset: 0,
    //                                 size: None,
    //                             }
    //                         ),
    //                     },
    //                 ],
    //                 label: Some("converter_bind_group"),
    //             })
    //         );
    //     }
}

// helper: допаковать, если stride != row
// fn copy_plane_8bit(
//     queue: &wgpu::Queue,
//     texture: &wgpu::Texture,
//     src: &[u8],
//     stride: u32,
//     w: u32,
//     h: u32,
//     row_bytes: u32,
// ) {
//     if stride == row_bytes {
//         queue.write_texture(
//             wgpu::TexelCopyTextureInfo {
//                 texture,
//                 mip_level: 0,
//                 origin: wgpu::Origin3d::ZERO,
//                 aspect: wgpu::TextureAspect::All,
//             },
//             src,
//             wgpu::TexelCopyBufferLayout {
//                 offset: 0,
//                 bytes_per_row: Some(row_bytes),
//                 rows_per_image: Some(h),
//             },
//             wgpu::Extent3d {
//                 width: w,
//                 height: h,
//                 depth_or_array_layers: 1,
//             },
//         );
//     } else {
//         assert_eq!(stride, w * 2);
//         panic!("Packing plane because stride != row");
//         let mut packed = Vec::with_capacity((row_bytes * h) as usize);
//         for y in 0..h {
//             let from = (y * stride) as usize;
//             let to = from + row_bytes as usize;
//             packed.extend_from_slice(&src[from..to]);
//         }
//         queue.write_texture(
//             wgpu::TexelCopyTextureInfo {
//                 texture,
//                 mip_level: 0,
//                 origin: wgpu::Origin3d::ZERO,
//                 aspect: wgpu::TextureAspect::All,
//             },
//             &packed,
//             wgpu::TexelCopyBufferLayout {
//                 offset: 0,
//                 bytes_per_row: Some(row_bytes),
//                 rows_per_image: Some(h),
//             },
//             wgpu::Extent3d {
//                 width: w,
//                 height: h,
//                 depth_or_array_layers: 1,
//             },
//         );
//     }
// }
//
// fn copy_plane_16bit(
//     queue: &wgpu::Queue,
//     texture: &wgpu::Texture,
//     src: &[u8],
//     stride: u32,    // байт на строку в исходных данных
//     w: u32,         // ширина текстуры в пикселях
//     h: u32,         // высота текстуры в пикселях
//     row_bytes: u32, // ожидаемая длина строки в байтах (w * 2)
// ) {
//     if stride == row_bytes {
//         // Прямая загрузка если stride совпадает
//         queue.write_texture(
//             wgpu::TexelCopyTextureInfo {
//                 texture,
//                 mip_level: 0,
//                 origin: wgpu::Origin3d::ZERO,
//                 aspect: wgpu::TextureAspect::All,
//             },
//             src,
//             wgpu::TexelCopyBufferLayout {
//                 offset: 0,
//                 bytes_per_row: Some(row_bytes),
//                 rows_per_image: Some(h),
//             },
//             wgpu::Extent3d {
//                 width: w,
//                 height: h,
//                 depth_or_array_layers: 1,
//             },
//         );
//     } else {
//         // Перепаковка данных
//         let mut packed = Vec::with_capacity((row_bytes * h) as usize);
//         for y in 0..h {
//             let from = (y * stride) as usize;
//             let to = from + row_bytes as usize;
//             if to <= src.len() {
//                 packed.extend_from_slice(&src[from..to]);
//             }
//         }
//         queue.write_texture(
//             wgpu::TexelCopyTextureInfo {
//                 texture,
//                 mip_level: 0,
//                 origin: wgpu::Origin3d::ZERO,
//                 aspect: wgpu::TextureAspect::All,
//             },
//             &packed,
//             wgpu::TexelCopyBufferLayout {
//                 offset: 0,
//                 bytes_per_row: Some(row_bytes),
//                 rows_per_image: Some(h),
//             },
//             wgpu::Extent3d {
//                 width: w,
//                 height: h,
//                 depth_or_array_layers: 1,
//             },
//         );
//     }
// }
//
// fn convert_10bit_to_16unorm(src: &[u8], width: u32, height: u32, stride: u32) -> Vec<u8> {
//     let mut result = Vec::with_capacity((width * height * 2) as usize);
//
//     for y in 0..height {
//         let row_start = (y * stride) as usize;
//         for x in 0..width {
//             let byte_offset = row_start + (x as usize * 2); // 2 bytes per 10-bit sample
//             if byte_offset + 1 < src.len() {
//                 // Читаем 16-битное little-endian значение
//                 let raw_value = u16::from_le_bytes([src[byte_offset], src[byte_offset + 1]]);
//                 // Преобразуем 10-битное в 16-битное UNORM
//                 let normalized = ((raw_value as f32) / 1023.0 * 65535.0) as u16;
//                 result.extend_from_slice(&normalized.to_le_bytes());
//             }
//         }
//     }
//
//     result
// }
