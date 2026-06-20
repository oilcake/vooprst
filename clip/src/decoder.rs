use std::path::PathBuf;

use crossbeam_channel::{Receiver, Sender};
use ffmpeg_next::util::frame::Video;

use crate::clip::Clip;
use transport::link::Link;

pub struct Decoder {
    clip: Clip,
    link: Link,
    files: Vec<PathBuf>,
    current_file_index: usize,
    frame_buffer: Sender<Video>,
    incoming_command: Receiver<Option<DecoderCommand>>,
}

pub enum DecoderCommand {
    NextFile,
    PreviousFile,
}

impl Decoder {
    pub fn new(
        clip: Clip,
        link: Link,
        files: Vec<PathBuf>,
        frame_buffer: Sender<Video>,
        command_receiver: Receiver<Option<DecoderCommand>>,
    ) -> Decoder {
        Decoder {
            clip,
            link,
            files,
            current_file_index: 0,
            frame_buffer,
            incoming_command: command_receiver,
        }
    }

    pub fn send_frame(&mut self) {
        self.link.update_phase_and_beat();
        let frame = self.clip.play_video_at_position(self.link.phase as f32);
        self.frame_buffer.send(frame).unwrap();
        if let Ok(Some(command)) = self.incoming_command.try_recv() {
            self.handle_command(command);
        }
    }

    fn handle_command(&mut self, command: DecoderCommand) {
        match command {
            DecoderCommand::NextFile => {
                self.current_file_index = (self.current_file_index + 1) % self.files.len();
                self.clip =
                    Clip::new(self.files[self.current_file_index].to_str().unwrap()).unwrap();
                self.clip.cache_all_frames().unwrap();
            }
            DecoderCommand::PreviousFile => {
                self.current_file_index =
                    (self.current_file_index + self.files.len() - 1) % self.files.len();
                self.clip =
                    Clip::new(self.files[self.current_file_index].to_str().unwrap()).unwrap();
                self.clip.cache_all_frames().unwrap();
            }
        }
    }
}
