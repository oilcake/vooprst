use std::collections::VecDeque;

use ffmpeg_next as ffmpeg;
use ffmpeg_next::util::frame::Video;
use tracing::{debug, info};

/// Ring buffer capacity in frames. Will move to config later.
const RING_CAPACITY: usize = 16;

/// Presentation timestamp in stream time-base units. Newtype so frame
/// timestamps can't be silently mixed with dts, durations or frame numbers.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Pts(i64);

/// On-demand decoding clip.
///
/// Holds no full-frame cache: a demux-only packet index maps frame numbers to
/// timestamps, decoded frames land in a small ring buffer, and any miss either
/// decodes forward (cheap for sequential playback) or seeks to the previous
/// keyframe and catches up (random access). Direction-agnostic by design —
/// callers just ask for frame numbers in whatever order they like.
pub struct Clip {
    ctx: ffmpeg::format::context::Input,
    video_stream_index: usize,
    video_time_base: ffmpeg::Rational,
    /// Presentation-order pts of every frame; position in vec == frame number.
    index: Vec<Pts>,
    decoder: ffmpeg::codec::decoder::Video,
    /// Recently decoded frames keyed by pts.
    ring: VecDeque<(Pts, Video)>,
    /// pts of the most recently decoded frame (decoder cursor position).
    cursor: Option<Pts>,
    /// Decoder has been sent EOF and not flushed since. Sending EOF twice
    /// without a flush in between is an AVERROR_EOF; track the state.
    decoder_eof: bool,
}

impl Clip {
    pub fn new(path: &str) -> Result<Clip, ffmpeg::Error> {
        let mut ctx = ffmpeg::format::input(&path)?;

        let (video_stream_index, video_time_base, parameters, meta_index) = {
            let input = ctx.streams().best(ffmpeg::media::Type::Video).unwrap();
            (
                input.index(),
                input.time_base(),
                input.parameters(),
                cfr_index(&input),
            )
        };

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(parameters)?;
        let decoder = context_decoder.decoder().video()?;

        // Frame-number → pts mapping. For CFR content it is arithmetic and
        // comes straight from the container sample table — instant, no IO.
        // Otherwise fall back to a demux scan (reads the whole file, slow).
        let index = match meta_index {
            Some(index) => {
                info!("CFR index from container: {} frames", index.len());
                index
            }
            None => {
                let mut index = Vec::new();
                for (stream, packet) in ctx.packets() {
                    if stream.index() == video_stream_index && let Some(pts) = packet.pts() {
                        index.push(Pts(pts));
                    }
                }
                index.sort_unstable();
                info!("Indexed {} frames by demux scan", index.len());
                index
            }
        };

        // No rewind needed: cursor is None, so the first frame_at() always seeks.

        Ok(Clip {
            ctx,
            video_stream_index,
            video_time_base,
            index,
            decoder,
            ring: VecDeque::with_capacity(RING_CAPACITY + 1),
            cursor: None,
            decoder_eof: false,
        })
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Frame at normalized position [0, 1) — the Link phase mapping.
    pub fn play_video_at_position(&mut self, position: f32) -> Video {
        let number = ((self.index.len() as f32) * position) as usize;
        let number = number.min(self.index.len() - 1);

        debug!("Frame {number}/{} at position {position}", self.index.len());

        self.frame_at(number).expect("failed to decode frame")
    }

    /// Frame by number. Ring hit → decode forward → seek + catch-up.
    pub fn frame_at(&mut self, number: usize) -> Result<Video, ffmpeg::Error> {
        let target = self.index[number];

        if let Some((_, frame)) = self.ring.iter().find(|(pts, _)| *pts == target) {
            return Ok(frame.clone());
        }

        // Small forward gap: decoding through is cheaper than a seek.
        // Large jump or any backward step: seek to previous keyframe.
        let decode_forward = match self.cursor {
            Some(cursor_pts) => match self.index.binary_search(&cursor_pts) {
                Ok(cursor_number) => {
                    number > cursor_number && number - cursor_number <= RING_CAPACITY
                }
                Err(_) => false,
            },
            None => false,
        };

        if !decode_forward {
            self.seek_to(target)?;
        }

        self.decode_until(target)
    }

    /// Seek so that decoding starts at the last keyframe with pts <= target.
    fn seek_to(&mut self, target_pts: Pts) -> Result<(), ffmpeg::Error> {
        // Input::seek goes through avformat_seek_file with stream_index -1,
        // so the timestamp must be in AV_TIME_BASE (microseconds), not in
        // stream time-base units — rescale, or the seek lands at file start.
        let ts = (target_pts.0 as i128 * self.video_time_base.numerator() as i128 * 1_000_000
            / self.video_time_base.denominator() as i128) as i64;
        debug!("Seek to pts {} ({ts}us)", target_pts.0);
        self.ctx.seek(ts, 0..ts)?;
        self.decoder.flush();
        self.cursor = None;
        self.decoder_eof = false;
        Ok(())
    }

    /// Decode from current position until `target_pts` is produced.
    /// Every intermediate frame goes into the ring.
    fn decode_until(&mut self, target_pts: Pts) -> Result<Video, ffmpeg::Error> {
        let mut decoded = Video::empty();

        for (stream, packet) in self.ctx.packets() {
            if stream.index() != self.video_stream_index {
                continue;
            }
            self.decoder.send_packet(&packet)?;

            while self.decoder.receive_frame(&mut decoded).is_ok() {
                if note_decoded(&mut self.ring, &mut self.cursor, &decoded, target_pts) {
                    return Ok(decoded.clone());
                }
            }
        }

        // Demuxer EOF: drain the decoder (once — repeated EOF without a
        // flush errors out, possibly leaving the tail frame stuck inside).
        if !self.decoder_eof {
            self.decoder.send_eof()?;
            self.decoder_eof = true;
        }
        while self.decoder.receive_frame(&mut decoded).is_ok() {
            if note_decoded(&mut self.ring, &mut self.cursor, &decoded, target_pts) {
                return Ok(decoded.clone());
            }
        }

        Err(ffmpeg::Error::Eof)
    }
}

/// CFR frame index from container metadata: pts(n) = start + n*step.
/// None when the container can't tell (VFR or missing sample table).
fn cfr_index(input: &ffmpeg::format::stream::Stream) -> Option<Vec<Pts>> {
    let frames = input.frames();
    let duration = input.duration();
    // AV_NOPTS_VALUE when unknown — fall back to 0-based timestamps.
    let start = match input.start_time() {
        i64::MIN => 0,
        start => start,
    };
    if frames <= 0 || duration <= 0 {
        return None;
    }
    let step = duration / frames;
    if step <= 0 {
        return None;
    }
    Some((0..frames).map(|n| Pts(start + n * step)).collect())
}

/// Record a freshly decoded frame into the ring. Returns true on target match.
fn note_decoded(
    ring: &mut VecDeque<(Pts, Video)>,
    cursor: &mut Option<Pts>,
    frame: &Video,
    target_pts: Pts,
) -> bool {
    let Some(pts) = frame.pts().map(Pts) else {
        return false;
    };
    *cursor = Some(pts);
    ring.push_back((pts, frame.clone()));
    if ring.len() > RING_CAPACITY {
        ring.pop_front();
    }
    // `>=` not `==`: decoder output is presentation-ordered (monotonic), and
    // a computed CFR target can deviate from the real pts by rounding.
    pts >= target_pts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_paths() -> Vec<String> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../samples");
        let mut v: Vec<_> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "mp4"))
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    fn sample_path() -> Option<String> {
        sample_paths().into_iter().next()
    }

    #[test]
    fn frame_at_ring_forward_and_seek_paths() {
        let Some(path) = sample_path() else {
            return;
        };
        ffmpeg_next::init().unwrap();
        let mut clip = Clip::new(&path).unwrap();
        assert!(clip.len() > 4);

        // First access seeks from scratch.
        assert_eq!(clip.frame_at(0).unwrap().pts().map(Pts), Some(clip.index[0]));
        // Sequential forward: decode-forward path.
        assert_eq!(clip.frame_at(1).unwrap().pts().map(Pts), Some(clip.index[1]));
        // Forward jump beyond RING_CAPACITY: seek path.
        let last = clip.len() - 1;
        assert_eq!(clip.frame_at(last).unwrap().pts().map(Pts), Some(clip.index[last]));
        // Backward jump: seek path.
        assert_eq!(clip.frame_at(2).unwrap().pts().map(Pts), Some(clip.index[2]));
        // Sequential again after the seek.
        assert_eq!(clip.frame_at(3).unwrap().pts().map(Pts), Some(clip.index[3]));
        // Repeat of a recent frame: ring hit.
        assert_eq!(clip.frame_at(3).unwrap().pts().map(Pts), Some(clip.index[3]));
    }

    #[test]
    fn sequential_loop_wraps() {
        // Link phase loops the clip: frames requested in order, then again
        // from 0. Decoder EOF state must survive the wrap (regression: the
        // last frame left the decoder EOF'd, the next request then crashed).
        ffmpeg_next::init().unwrap();
        for path in sample_paths() {
            let mut clip = Clip::new(&path).unwrap();
            for _ in 0..2 {
                for i in 0..clip.len() {
                    assert_eq!(clip.frame_at(i).unwrap().pts().map(Pts), Some(clip.index[i]));
                }
            }
        }
    }
}


