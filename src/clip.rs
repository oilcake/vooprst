use ffmpeg_next as ffmpeg;
use log::{info, debug};

pub struct Size {
    width: u32,
    height: u32,
}

impl Size {
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn width(&self) -> u32 {
        self.width
    }
}

pub struct Clip {
    ctx: ffmpeg::format::context::Input,
    video_stream_index: usize,
    total_frames: f32,
    decoder: ffmpeg::codec::decoder::Video,
    scaler: ffmpeg::software::scaling::Context,
    pub size: Size,
    frames: Vec<ffmpeg::util::frame::Video>,
}

impl Clip {
    pub fn new(path: &str) -> Result<Clip, ffmpeg::Error> {
        let ctx = ffmpeg::format::input(&path)?;
        let input = ctx.streams().best(ffmpeg::media::Type::Video).unwrap();
        let video_stream_index = input.index();

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
        let decoder = context_decoder.decoder().video()?;
        let width = decoder.width();
        let height = decoder.height();

        let total_frames = input.frames() as f32;
        let scaler = ffmpeg::software::scaling::Context::get(
            decoder.format(),
            width,
            height,
            ffmpeg::format::Pixel::RGBA,
            width,
            height,
            ffmpeg::software::scaling::Flags::BILINEAR,
        )
        .unwrap();

        Ok(Clip {
            ctx,
            video_stream_index,
            total_frames,
            decoder,
            scaler,
            size: Size { width, height },
            frames: Vec::new(),
        })
    }

    pub fn play_video_at_position(&mut self, position: f32) -> ffmpeg::util::frame::Video {
        let frame_number = self.total_frames as f32 * position;

        debug!("Getting frame {frame_number} from {} of frames, at position {position}", self.total_frames);

        self.frames[frame_number as usize].clone()
    }
    pub fn cache_all_frames(&mut self) -> Result<(), ffmpeg::Error> {
        // let mut packet = ffmpeg::Packet::empty();
        let mut decoded = ffmpeg::util::frame::Video::empty();

        // Read packets from the input file
        for (stream, packet) in self.ctx.packets() {
            if stream.index() != self.video_stream_index {
                continue;
            }
            info!("Reading packet from stream {}", stream.index());

            // Send the packet to the decoder
            self.decoder.send_packet(&packet)?;

            // Receive all frames the decoder can produce from this packet
            while self.decoder.receive_frame(&mut decoded).is_ok() {
                // Clone the frame and store it (Video frame doesn't implement Copy)
                let mut rgb = ffmpeg::util::frame::Video::empty();
                rgb.set_format(ffmpeg::format::Pixel::RGBA);
                rgb.set_width(self.size.width);
                rgb.set_height(self.size.height);
                self.scaler.run(&decoded, &mut rgb)?;
                self.frames.push(rgb);
            }
        }

        // Flush the decoder
        self.decoder.send_eof()?;
        while self.decoder.receive_frame(&mut decoded).is_ok() {
            let mut rgb = ffmpeg::util::frame::Video::empty();
            rgb.set_format(ffmpeg::format::Pixel::RGBA);
            rgb.set_width(self.size.width);
            rgb.set_height(self.size.height);
            self.scaler.run(&decoded, &mut rgb)?;
            self.frames.push(rgb);
        }
        info!("Cached {} frames", self.frames.len());
        Ok(())
    }
}
