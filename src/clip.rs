use ffmpeg_next as ffmpeg;

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
        })
    }

    pub fn play_video_at_position(
        &mut self,
        position: f32,
    ) -> Result<ffmpeg::util::frame::Video, ffmpeg::Error> {
        let stream = self.ctx.stream(self.video_stream_index).unwrap();

        let duration = stream.duration(); // In stream timebase units
        let target_ts = (duration as f32 * position).round() as i64;
        let seek_ts = target_ts.max(0);

        // Seek to the timestamp (in stream's timebase)
        self.ctx.seek(seek_ts, ..)?;

        let mut frame = ffmpeg::util::frame::Video::empty();

        for (stream, packet) in self.ctx.packets() {
            if stream.index() == self.video_stream_index {
                self.decoder.send_packet(&packet)?;

                while self.decoder.receive_frame(&mut frame).is_ok() {
                    if frame.timestamp().unwrap_or(0) >= seek_ts {
                        let mut yuv_frame = ffmpeg::util::frame::Video::empty();
                        self.scaler.run(&frame, &mut yuv_frame)?;
                        return Ok(yuv_frame);
                    }
                }
            }
        }

        Err(ffmpeg::Error::StreamNotFound)
    }
}
