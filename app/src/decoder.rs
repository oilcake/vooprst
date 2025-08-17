use std::path::PathBuf;

use crossbeam_channel::Sender;
use ffmpeg_next::util::frame::Video;

use crate::clip::Clip;
use transport::link::Link;

pub struct Decoder {
    clip: Clip,
    link: Link,
    files: Vec<PathBuf>,
    current_file_index: usize,
    frame_buffer: Sender<Video>
}

impl Decoder {
    pub fn new(clip: Clip, link: Link, files: Vec<PathBuf>, frame_buffer: Sender<Video>) -> Decoder {
        Decoder {
            clip,
            link,
            files,
            current_file_index: 0,
            frame_buffer
        }
    }
    pub fn send_frame(&mut self) {
            self.link.update_phase_and_beat();
            let frame = self.clip.play_video_at_position(self.link.phase as f32);
            self.frame_buffer.send(frame).unwrap();
    }
}
