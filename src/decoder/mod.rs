use either::Either;
use ffmpeg_next::{codec::Context, decoder, format::context::Input, Packet, Stream};
use iter::DecodeIter;
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
    FfmpegError(ffmpeg_next::Error),
    #[error("no frames yet to decode")]
    NoFramesYet,
    #[error("failed to convert frame to image")]
    ImageError,
    #[error("audio frame had a length not divisible by 4")]
    AudioFrameLength,
    #[error("there was no stream of the requested type: {0}")]
    NoSuchStream(&'static str),
}

impl From<ffmpeg_next::Error> for DecodeError {
    fn from(value: ffmpeg_next::Error) -> Self {
        match value {
            ffmpeg_next::Error::Other { errno: 11 } => Self::NoFramesYet,
            e => Self::FfmpegError(e),
        }
    }
}

pub enum Decoder {
    VideoOnly {
        video_decoder: decoder::Video,
        video_stream_idx: usize,
        resolution_hint: ResolutionHint,
    },
    AudioOnly {
        audio_decoder: decoder::Audio,
        audio_stream_idx: usize,
    },
    Both {
        video_decoder: decoder::Video,
        video_stream_idx: usize,
        resolution_hint: ResolutionHint,
        audio_decoder: decoder::Audio,
        audio_stream_idx: usize,
    },
}

impl Decoder {
    pub fn new_both(
        video_stream: Stream,
        audio_stream: Stream,
        resolution_hint: ResolutionHint,
    ) -> Result<Self, DecodeError> {
        let video_ctx = Context::from_parameters(video_stream.parameters())?;
        let audio_ctx = Context::from_parameters(audio_stream.parameters())?;
        Ok(Self::Both {
            video_decoder: video_ctx.decoder().video()?,
            video_stream_idx: video_stream.index(),
            resolution_hint,
            audio_decoder: audio_ctx.decoder().audio()?,
            audio_stream_idx: audio_stream.index(),
        })
    }

    pub fn new_audio_only(audio_stream: Stream) -> Result<Self, DecodeError> {
        let audio_ctx = Context::from_parameters(audio_stream.parameters())?;
        Ok(Self::AudioOnly {
            audio_decoder: audio_ctx.decoder().audio()?,
            audio_stream_idx: audio_stream.index(),
        })
    }

    pub fn new_video_only(
        video_stream: Stream,
        resolution_hint: ResolutionHint,
    ) -> Result<Self, DecodeError> {
        let video_ctx = Context::from_parameters(video_stream.parameters())?;
        Ok(Self::VideoOnly {
            video_decoder: video_ctx.decoder().video()?,
            video_stream_idx: video_stream.index(),
            resolution_hint,
        })
    }

    /// sends packet to approptiate decoder, otherwise discards it
    pub fn send_packet(&mut self, packet: &Packet) -> Result<(), ffmpeg_next::Error> {
        let packet_stream_idx = packet.stream();
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

    pub fn try_receive_any_frame(&mut self) -> Result<Either<VideoFrame, AudioFrame>, DecodeError> {
        match self {
            Self::VideoOnly {
                video_decoder: _,
                video_stream_idx: _,
                resolution_hint: _,
            } => self.try_receive_video_frame().map(Either::Left),
            Self::AudioOnly {
                audio_decoder: _,
                audio_stream_idx: _,
            } => self.try_receive_audio_frame().map(Either::Right),
            Self::Both {
                video_decoder,
                video_stream_idx,
                resolution_hint,
                audio_decoder,
                audio_stream_idx,
            } => self.try_receive_video_frame().map(Either::Left),
        }
    }

    pub fn try_receive_video_frame(&mut self) -> Result<VideoFrame, DecodeError> {
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

    pub fn try_receive_audio_frame(&mut self) -> Result<AudioFrame, DecodeError> {
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

    pub fn into_frame_iter(self, input: Input) -> DecodeIter {
        DecodeIter {
            input,
            decoders: self,
        }
    }
}
