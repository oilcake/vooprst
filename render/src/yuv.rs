use ffmpeg_next::{format::Pixel, frame::Video};
use tracing::debug;

/// Format tag passed to compute shader.
/// Values must match `converter.wgsl` switch cases.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum YuvFormat {
    YUV420P = 0,
    YUV422P = 1,
    YUV444P = 2,
    YUV422P10LE = 3,
    YUVA444P12LE = 4,
    YUV422P12LE = 5,
}

impl YuvFormat {
    pub fn from_pixel(pix: Pixel) -> Option<Self> {
        match pix {
            Pixel::YUV420P => Some(Self::YUV420P),
            Pixel::YUV422P => Some(Self::YUV422P),
            Pixel::YUV444P => Some(Self::YUV444P),
            Pixel::YUV422P10LE => Some(Self::YUV422P10LE),
            Pixel::YUVA444P12LE => Some(Self::YUVA444P12LE),
            Pixel::YUV422P12LE => Some(Self::YUV422P12LE),
            _ => None,
        }
    }

    /// Bytes per component in the raw ffmpeg frame data.
    fn component_size(&self) -> usize {
        match self {
            Self::YUV420P | Self::YUV422P | Self::YUV444P => 1, // u8
            Self::YUV422P10LE | Self::YUVA444P12LE | Self::YUV422P12LE => 2, // u16 LE
        }
    }
}

/// One plane's texture plus its dimensions.
pub struct Plane {
    pub texture: wgpu::Texture,
    pub width: u32,
    pub height: u32,
}

/// Three-plane YUV data on GPU.
pub struct YuvPlanes {
    pub y: Plane,
    pub u: Plane,
    pub v: Plane,
    #[allow(dead_code)]
    pub format: YuvFormat,
}

impl YuvPlanes {
    /// Create GPU textures sized for `frame`'s format.
    /// 8-bit formats use R8Unorm, >8-bit use R16Unorm — raw upload, no CPU conversion.
    /// 10-bit data is scaled to full range in the compute shader instead.
    pub fn new(device: &wgpu::Device, frame: &Video, format: YuvFormat) -> Self {
        let (y_w, y_h) = (frame.width(), frame.height());
        let (uv_w, uv_h) = chroma_size(y_w, y_h, format);

        let tex_format = match format.component_size() {
            1 => wgpu::TextureFormat::R8Unorm,
            _ => wgpu::TextureFormat::R16Unorm,
        };
        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;

        let make_plane = |label: &str, w: u32, h: u32| -> Plane {
            Plane {
                texture: device.create_texture(&wgpu::TextureDescriptor {
                    size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: tex_format,
                    usage,
                    label: Some(label),
                    view_formats: &[],
                }),
                width: w,
                height: h,
            }
        };

        Self {
            y: make_plane("y_plane", y_w, y_h),
            u: make_plane("u_plane", uv_w, uv_h),
            v: make_plane("v_plane", uv_w, uv_h),
            format,
        }
    }

    /// Default texture views for all three planes.
    pub fn views(&self) -> (wgpu::TextureView, wgpu::TextureView, wgpu::TextureView) {
        (
            self.y.texture.create_view(&Default::default()),
            self.u.texture.create_view(&Default::default()),
            self.v.texture.create_view(&Default::default()),
        )
    }

    /// Upload all three planes from `frame` to GPU.
    /// Raw passthrough of ffmpeg data; stride handles row padding.
    pub fn upload(&self, queue: &wgpu::Queue, frame: &Video) {
        upload_plane(queue, &self.y, frame.data(0), frame.stride(0));
        upload_plane(queue, &self.u, frame.data(1), frame.stride(1));
        upload_plane(queue, &self.v, frame.data(2), frame.stride(2));
        debug!(
            "Uploaded YUV planes: Y={}x{} U={}x{} V={}x{}",
            self.y.width, self.y.height, self.u.width, self.u.height, self.v.width, self.v.height
        );
    }
}

/// UV plane dimensions for a given luma size and format.
fn chroma_size(luma_w: u32, luma_h: u32, fmt: YuvFormat) -> (u32, u32) {
    match fmt {
        YuvFormat::YUV420P => (luma_w / 2, luma_h / 2),
        YuvFormat::YUV422P | YuvFormat::YUV422P10LE | YuvFormat::YUV422P12LE => (luma_w / 2, luma_h),
        YuvFormat::YUV444P | YuvFormat::YUVA444P12LE => (luma_w, luma_h),
    }
}

/// Upload plane data as-is. ffmpeg linesize (bytes per row, incl. padding)
/// is passed as the source layout stride. 10-bit LE u16 pairs map directly
/// onto R16Unorm texels on little-endian hosts.
fn upload_plane(queue: &wgpu::Queue, plane: &Plane, data: &[u8], stride: usize) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &plane.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(stride as u32),
            rows_per_image: Some(plane.height),
        },
        wgpu::Extent3d {
            width: plane.width,
            height: plane.height,
            depth_or_array_layers: 1,
        },
    );
}