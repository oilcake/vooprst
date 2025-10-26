use ffmpeg_next::{format::Pixel, frame::Video};

pub struct Plane {
    pub texture: wgpu::Texture,
    pub size: wgpu::Extent3d,
}

pub struct YUV {
    pub y: Plane,
    pub u: Plane,
    pub v: Plane,
    pub format: wgpu::TextureFormat,
}

impl YUV {
    pub fn adapt_size(&mut self, frame: &Video) {
        match frame.format() {
            Pixel::YUV420P => {
                self.y.size = wgpu::Extent3d {
                    width: frame.width(),
                    height: frame.height(),
                    depth_or_array_layers: 1,
                };
                self.u.size = wgpu::Extent3d {
                    width: frame.width() / 2,
                    height: frame.height() / 2,
                    depth_or_array_layers: 1,
                };
                self.v.size = self.u.size;
                self.format = wgpu::TextureFormat::R8Unorm;
            }
            Pixel::YUV422P10LE | Pixel::YUV422P12LE => {
                self.y.size = wgpu::Extent3d {
                    width: frame.width(),
                    height: frame.height(),
                    depth_or_array_layers: 1,
                };
                self.u.size = wgpu::Extent3d {
                    width: frame.width() / 2,
                    height: frame.height(),
                    depth_or_array_layers: 1,
                };
                self.v.size = self.u.size;
                self.format = wgpu::TextureFormat::R16Unorm;
            }
            Pixel::YUV444P => {
                self.y.size = wgpu::Extent3d {
                    width: frame.width(),
                    height: frame.height(),
                    depth_or_array_layers: 1,
                };
                self.u.size = wgpu::Extent3d {
                    width: frame.width(),
                    height: frame.height(),
                    depth_or_array_layers: 1,
                };
                self.v.size = self.u.size;
                self.format = wgpu::TextureFormat::R8Unorm;
            }
            _ => panic!("unsupported format: {:?}", frame.format()),
        }
    }
}
