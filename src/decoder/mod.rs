use either::Either;
use iter::DecodeIter;
use scuffle_ffmpeg::{
    consts::Const,
    decoder,
    error::{FfmpegError, FfmpegErrorCode},
    io::Input,
    packet::Packet,
    stream::Stream,
};
use util::{audio_from_decoder, image_from_decoder};

use crate::{
    dimensions::ResolutionHint,
    frame::{AudioFrame, VideoFrame},
};

pub mod iter;
mod util;

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error(transparent)]
    FfmpegError(FfmpegError),
    #[error("no frames yet to decode")]
    NoFramesYet,
    #[error("failed to convert frame to image")]
    ImageError,
    #[error("audio frame had a length not divisible by 4")]
    AudioFrameLength,
    #[error("provided stream did not match the required type")]
    WrongStreamKind,
    #[error("there was no stream of the requested type: {0}")]
    NoSuchStream(&'static str),
}

impl From<FfmpegError> for DecodeError {
    fn from(value: FfmpegError) -> Self {
        match value {
            FfmpegError::Code(FfmpegErrorCode(-11)) => Self::NoFramesYet,
            e => Self::FfmpegError(e),
        }
    }
}

pub enum Decoder {
    VideoOnly {
        video_decoder: decoder::VideoDecoder,
        video_stream_idx: i32,
        resolution_hint: ResolutionHint,
    },
    AudioOnly {
        audio_decoder: decoder::AudioDecoder,
        audio_stream_idx: i32,
    },
    Both {
        video_decoder: decoder::VideoDecoder,
        video_stream_idx: i32,
        resolution_hint: ResolutionHint,
        audio_decoder: decoder::AudioDecoder,
        audio_stream_idx: i32,
    },
}

impl Decoder {
    pub fn new_maybe(
        video_stream: Option<Const<'_, Stream>>,
        audio_stream: Option<Const<'_, Stream>>,
        resolution_hint: ResolutionHint,
    ) -> Result<Self, DecodeError> {
        match (video_stream, audio_stream) {
            (Some(v), Some(a)) => Self::new_both(v, a, resolution_hint),
            (Some(v), None) => Self::new_video_only(v, resolution_hint),
            (None, Some(a)) => Self::new_audio_only(a),
            (None, None) => Err(DecodeError::NoSuchStream("audio or video")),
        }
    }
    pub fn new_both(
        video_stream: Const<'_, Stream>,
        audio_stream: Const<'_, Stream>,
        resolution_hint: ResolutionHint,
    ) -> Result<Self, DecodeError> {
        let vid_dec = decoder::Decoder::new(&video_stream)?
            .video()
            .map_err(|_| DecodeError::WrongStreamKind)?;
        let aud_dec = decoder::Decoder::new(&audio_stream)?
            .audio()
            .map_err(|_| DecodeError::WrongStreamKind)?;
        Ok(Self::Both {
            video_decoder: vid_dec,
            video_stream_idx: video_stream.index(),
            resolution_hint,
            audio_decoder: aud_dec,
            audio_stream_idx: audio_stream.index(),
        })
    }

    pub fn new_audio_only(audio_stream: Const<'_, Stream>) -> Result<Self, DecodeError> {
        let aud_dec = decoder::Decoder::new(&audio_stream)?
            .audio()
            .map_err(|_| DecodeError::WrongStreamKind)?;
        Ok(Self::AudioOnly {
            audio_decoder: aud_dec,
            audio_stream_idx: audio_stream.index(),
        })
    }

    pub fn new_video_only(
        video_stream: Const<'_, Stream>,
        resolution_hint: ResolutionHint,
    ) -> Result<Self, DecodeError> {
        let vid_dec = decoder::Decoder::new(&video_stream)?
            .video()
            .map_err(|_| DecodeError::WrongStreamKind)?;
        Ok(Self::VideoOnly {
            video_decoder: vid_dec,
            video_stream_idx: video_stream.index(),
            resolution_hint,
        })
    }

    /// sends packet to approptiate decoder, otherwise discards it
    pub fn send_packet(&mut self, packet: &Packet) -> Result<(), FfmpegError> {
        let packet_stream_idx = packet.stream_index();
        match self {
            Self::VideoOnly {
                video_decoder,
                video_stream_idx,
                resolution_hint: _,
            } => {
                if packet_stream_idx != *video_stream_idx {
                    return Ok(());
                }
                video_decoder.send_packet(packet)
            }
            Self::AudioOnly {
                audio_decoder,
                audio_stream_idx,
            } => {
                if packet_stream_idx != *audio_stream_idx {
                    return Ok(());
                }
                audio_decoder.send_packet(packet)
            }
            Self::Both {
                video_decoder,
                video_stream_idx,
                resolution_hint: _,
                audio_decoder,
                audio_stream_idx,
            } => {
                if packet_stream_idx == *video_stream_idx {
                    return video_decoder.send_packet(packet);
                }
                if packet_stream_idx == *audio_stream_idx {
                    return audio_decoder.send_packet(packet);
                }
                Ok(())
            }
        }
    }

    pub fn send_eof(&mut self) -> Result<(), FfmpegError> {
        match self {
            Decoder::VideoOnly {
                video_decoder,
                video_stream_idx: _,
                resolution_hint: _,
            } => {
                video_decoder.send_eof()?;
            }
            Decoder::AudioOnly {
                audio_decoder,
                audio_stream_idx: _,
            } => {
                audio_decoder.send_eof()?;
            }
            Decoder::Both {
                video_decoder,
                video_stream_idx: _,
                resolution_hint: _,
                audio_decoder,
                audio_stream_idx: _,
            } => {
                video_decoder.send_eof()?;
                audio_decoder.send_eof()?;
            }
        }
        Ok(())
    }

    pub fn try_receive_any_frame(
        &mut self,
    ) -> Result<Option<Either<VideoFrame, AudioFrame>>, DecodeError> {
        match self {
            Self::VideoOnly {
                video_decoder: _,
                video_stream_idx: _,
                resolution_hint: _,
            } => self.try_receive_video_frame().map(|r| r.map(Either::Left)),
            Self::AudioOnly {
                audio_decoder: _,
                audio_stream_idx: _,
            } => self.try_receive_audio_frame().map(|r| r.map(Either::Right)),
            Self::Both {
                video_decoder: _,
                video_stream_idx: _,
                resolution_hint: _,
                audio_decoder: _,
                audio_stream_idx: _,
            } => {
                let aud = self.try_receive_audio_frame().map(|r| r.map(Either::Right));
                if aud.as_ref().is_err() || aud.as_ref().unwrap().is_some() {
                    aud
                } else {
                    self.try_receive_video_frame().map(|r| r.map(Either::Left))
                }
            }
        }
    }

    pub fn try_receive_video_frame(&mut self) -> Result<Option<VideoFrame>, DecodeError> {
        match self {
            Self::VideoOnly {
                video_decoder,
                video_stream_idx: _,
                resolution_hint,
            } => image_from_decoder(video_decoder, resolution_hint),
            Self::AudioOnly {
                audio_decoder: _,
                audio_stream_idx: _,
            } => Err(DecodeError::NoSuchStream("video")),
            Self::Both {
                video_decoder,
                video_stream_idx: _,
                resolution_hint,
                audio_decoder: _,
                audio_stream_idx: _,
            } => image_from_decoder(video_decoder, resolution_hint),
        }
    }

    pub fn try_receive_audio_frame(&mut self) -> Result<Option<AudioFrame>, DecodeError> {
        match self {
            Self::VideoOnly {
                video_decoder: _,
                video_stream_idx: _,
                resolution_hint: _,
            } => Err(DecodeError::NoSuchStream("video")),
            Self::AudioOnly {
                audio_decoder,
                audio_stream_idx: _,
            } => audio_from_decoder(audio_decoder),
            Self::Both {
                video_decoder: _,
                video_stream_idx: _,
                resolution_hint: _,
                audio_decoder,
                audio_stream_idx: _,
            } => audio_from_decoder(audio_decoder),
        }
    }

    pub fn into_frame_iter<T: Send + Sync>(self, input: Input<T>) -> DecodeIter<T> {
        DecodeIter {
            input,
            decoders: self,
            eof_reached: false,
        }
    }
}
