use ffmpeg_next::util::frame::video::Video;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::video::{Window, WindowContext};

pub struct SdlVideoDisplay {
    canvas: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,
    width: u32,
    height: u32
}

impl SdlVideoDisplay {
    pub fn new(video_subsystem: &sdl2::VideoSubsystem, width: u32, height: u32) -> Self {
        let window = video_subsystem
            .window("Simple Video Player", width, height)
            .position_centered()
            .build()
            .unwrap();

        let canvas = window.into_canvas().build().unwrap();
        let texture_creator: TextureCreator<WindowContext> = canvas.texture_creator();

        Self {
            canvas,
            texture_creator,
            width,
            height
        }
    }

    pub fn display_frame(&mut self, frame: &Video) {
        // Upload YUV frame to texture
        let mut texture = self
            .texture_creator
            .create_texture(
                PixelFormatEnum::IYUV,
                sdl2::render::TextureAccess::Streaming,
                self.width,
                self.height,
            )
            .unwrap();
        texture
            .update_yuv(
                None,
                frame.data(0),
                frame.stride(0),
                frame.data(1),
                frame.stride(1),
                frame.data(2),
                frame.stride(2),
            )
            .unwrap();

        // Render to canvas
        self.canvas.clear();
        self.canvas.copy(&texture, None, None).unwrap();
        self.canvas.present();
    }
}
