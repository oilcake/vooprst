use crate::compute_converter::{ComputeConverter, ConverterParams};
use crate::yuv::{YuvFormat, YuvPlanes};
use ffmpeg_next::{color::Space, util::frame::Video};
use tracing::debug;

/// YCbCr matrix tag for the shader. 0 = BT.601, 1 = BT.709.
/// Unspecified/unknown defaults to BT.709 (modern content).
fn color_matrix(frame: &Video) -> u32 {
    match frame.color_space() {
        Space::BT470BG | Space::SMPTE170M | Space::FCC => 0,
        _ => 1,
    }
}

/// Uploads YUV planes → compute-shader → RGBA8 texture.
/// Owns plane textures, output texture, bind groups, dispatch.
pub struct Converter {
    compute_converter: ComputeConverter,
    converter_bind_group: Option<wgpu::BindGroup>,
    params_buffer: wgpu::Buffer,

    /// Current YUV plane textures on GPU.
    yuv_planes: Option<YuvPlanes>,
    /// Output texture (sampled by display pipeline).
    universal_texture: Option<wgpu::Texture>,

    video_width: u32,
    video_height: u32,
}

impl Converter {
    pub fn new(device: &wgpu::Device) -> Self {
        let compute_converter = ComputeConverter::new(device);
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("converter_params_buffer"),
            size: std::mem::size_of::<ConverterParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            compute_converter,
            converter_bind_group: None,
            params_buffer,
            yuv_planes: None,
            universal_texture: None,
            video_width: 0,
            video_height: 0,
        }
    }

    /// Process a video frame: upload planes, run compute shader, return output texture.
    pub fn process_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &Video,
    ) -> &wgpu::Texture {
        let format = YuvFormat::from_pixel(frame.format())
            .unwrap_or_else(|| panic!("unsupported pixel format: {:?}", frame.format()));

        let width = frame.width();
        let height = frame.height();

        // Recreate textures on size change
        if self.universal_texture.is_none()
            || self.video_width != width
            || self.video_height != height
        {
            self.video_width = width;
            self.video_height = height;
            self.recreate_universal_texture(device, width, height);
            self.yuv_planes = Some(YuvPlanes::new(device, frame, format));
        }

        let planes = self.yuv_planes.as_ref().unwrap();
        planes.upload(queue, frame);

        let (y_view, u_view, v_view) = planes.views();
        let out_view = self.universal_texture.as_ref().unwrap().create_view(&Default::default());

        self.setup_bind_group(device, &y_view, &u_view, &v_view, &out_view);
        self.update_params(queue, format, width, height, color_matrix(frame));
        self.dispatch(device, queue, width, height);

        self.universal_texture.as_ref().unwrap()
    }

    pub fn universal_texture(&self) -> Option<&wgpu::Texture> {
        self.universal_texture.as_ref()
    }

    fn recreate_universal_texture(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("universal_texture"),
            view_formats: &[],
        });
        self.universal_texture = Some(texture);
        debug!("Created universal texture: {}x{}", width, height);
    }

    fn setup_bind_group(
        &mut self,
        device: &wgpu::Device,
        y_view: &wgpu::TextureView,
        u_view: &wgpu::TextureView,
        v_view: &wgpu::TextureView,
        out_view: &wgpu::TextureView,
    ) {
        self.converter_bind_group =
            Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.compute_converter.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(y_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(u_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(v_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(out_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.params_buffer,
                            offset: 0,
                            size: None,
                        }),
                    },
                ],
                label: Some("converter_bind_group"),
            }));
    }

    fn update_params(
        &mut self,
        queue: &wgpu::Queue,
        format: YuvFormat,
        width: u32,
        height: u32,
        colorspace: u32,
    ) {
        let params = ConverterParams {
            format: format as u32,
            width,
            height,
            colorspace,
        };
        queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
    }

    fn dispatch(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) {
        let wg_x = (width + 7) / 8;
        let wg_y = (height + 7) / 8;
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut cpass = encoder.begin_compute_pass(&Default::default());
            cpass.set_pipeline(&self.compute_converter.pipeline);
            cpass.set_bind_group(0, self.converter_bind_group.as_ref().unwrap(), &[]);
            cpass.dispatch_workgroups(wg_x, wg_y, 1);
        }
        queue.submit(Some(encoder.finish()));
        debug!("Dispatched compute: {}x{} workgroups", wg_x, wg_y);
    }
}