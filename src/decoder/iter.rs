use either::Either;

use crate::frame::{AudioFrame, VideoFrame};

use super::{DecodeError, Decoder};

pub struct DecodeIter {
    pub(super) input: ffmpeg_next::format::context::Input,
    pub(super) decoders: Decoder,
}

impl Iterator for DecodeIter {
    type Item = Result<Either<VideoFrame, AudioFrame>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((_, packet)) = self.input.packets().next() {
            if self.decoders.send_packet(&packet).is_err() {
                return None;
            }
            return Some(self.decoders.try_receive_any_frame());
        }

        None
    }
}
