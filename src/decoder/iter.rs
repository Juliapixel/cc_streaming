use either::Either;
use scuffle_ffmpeg::io::Input;

use crate::frame::{AudioFrame, VideoFrame};

use super::{DecodeError, Decoder};

pub struct DecodeIter<T: Send + Sync> {
    pub(super) input: Input<T>,
    pub(super) decoders: Decoder,
    pub(super) eof_reached: bool,
}

impl<T: Send + Sync> Iterator for DecodeIter<T> {
    type Item = Result<Either<VideoFrame, AudioFrame>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.input.packets().next() {
                Some(Ok(packet)) => {
                    if let Err(e) = self.decoders.send_packet(&packet) {
                        return Some(Err(e.into()));
                    }
                    match self.decoders.try_receive_any_frame() {
                        Ok(Some(frame)) => return Some(Ok(frame)),
                        Ok(None) => continue,
                        Err(e) => return Some(Err(e)),
                    };
                }
                Some(Err(e)) => return Some(Err(e.into())),
                None => {
                    if !self.eof_reached {
                        if let Err(e) = self.decoders.send_eof() {
                            return Some(Err(e.into()));
                        };
                    }
                    self.eof_reached = true;
                }
            }
        }
    }
}
