use either::Either;
use ffmpeg_next::decoder;

use crate::frame::{AudioFrame, VideoFrame};

use super::{
    util::{audio_from_decoder, image_from_decoder},
    DecodeError,
};

pub struct DecodeIter {
    pub(super) input: ffmpeg_next::format::context::Input,
    pub(super) video_stream_idx: usize,
    pub(super) video_decoder: decoder::Video,
    pub(super) audio_stream_idx: usize,
    pub(super) audio_decoder: decoder::Audio,
    pub(super) target_height: u32,
}

impl Iterator for DecodeIter {
    type Item = Result<Either<VideoFrame, AudioFrame>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        for (stream, packet) in self.input.packets() {
            if stream.index() == self.video_stream_idx {
                self.video_decoder.send_packet(&packet).ok()?;
                return Some(
                    image_from_decoder(&mut self.video_decoder, todo!(), self.target_height)
                        .map(Either::Left),
                );
            }
            if stream.index() == self.audio_stream_idx {
                self.audio_decoder.send_packet(&packet).ok()?;
                return Some(audio_from_decoder(&mut self.audio_decoder).map(Either::Right));
            }
        }

        None
    }
}
