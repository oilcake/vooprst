use ffmpeg_next as ffmpeg;
use tracing::{info, debug};

pub struct Clip {
    ctx: ffmpeg::format::context::Input,
    video_stream_index: usize,
    total_frames: f32,
    decoder: ffmpeg::codec::decoder::Video,
    frames: Vec<ffmpeg::util::frame::Video>,
}

impl Clip {
    pub fn new(path: &str) -> Result<Clip, ffmpeg::Error> {
        let ctx = ffmpeg::format::input(&path)?;
        let input = ctx.streams().best(ffmpeg::media::Type::Video).unwrap();
        let video_stream_index = input.index();

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
        let decoder = context_decoder.decoder().video()?;

        let total_frames = input.frames() as f32;

        Ok(Clip {
            ctx,
            video_stream_index,
            total_frames,
            decoder,
            frames: Vec::new(),
        })
    }

    pub fn play_video_at_position(&self, position: f32) -> ffmpeg::util::frame::Video {
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
                self.frames.push(decoded.clone());
            }
        }

        // Flush the decoder
        self.decoder.send_eof()?;
        while self.decoder.receive_frame(&mut decoded).is_ok() {
            self.frames.push(decoded.clone());
        }
        info!("Cached {} frames", self.frames.len());
        Ok(())
    }
}
