use either::Either;

use crate::frame::{AudioFrame, VideoFrame};

use super::{
    DecodeError, EitherOrBoth,
};

pub struct DecodeIter {
    pub(super) input: ffmpeg_next::format::context::Input,
    pub(super) decoders: EitherOrBoth,
    pub(super) target_height: u32,
}

impl Iterator for DecodeIter {
    type Item = Result<Either<VideoFrame, AudioFrame>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((_, packet)) = self.input.packets().next() {
            self.decoders.send_packet(&packet);
            return Some(self.decoders.try_receive_any_frame())
        }

        None
    }
}
