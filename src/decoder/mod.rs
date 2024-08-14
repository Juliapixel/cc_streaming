use either::Either;
use ffmpeg_next::{
    codec::Context, decoder, format::context::Input, Packet, Stream
};
use iter::DecodeIter;
use util::{audio_from_decoder, image_from_decoder};

use crate::frame::{AudioFrame, VideoFrame};

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
    NoSuchStream(&'static str)
}

impl From<ffmpeg_next::Error> for DecodeError {
    fn from(value: ffmpeg_next::Error) -> Self {
        match value {
            ffmpeg_next::Error::Other { errno: 11 } => Self::NoFramesYet,
            e => Self::FfmpegError(e),
        }
    }
}

enum EitherOrBoth {
    VideoOnly {
        video_decoder: decoder::Video,
        video_stream_idx: usize,
    },
    AudioOnly {
        audio_decoder: decoder::Audio,
        audio_stream_idx: usize,
    },
    Both {
        video_decoder: decoder::Video,
        video_stream_idx: usize,
        audio_decoder: decoder::Audio,
        audio_stream_idx: usize,
    }
}

impl EitherOrBoth {
    pub fn new_both(video_stream: Stream, audio_stream: Stream) -> Result<Self, DecodeError> {
        let video_ctx = Context::from_parameters(video_stream.parameters())?;
        let audio_ctx = Context::from_parameters(audio_stream.parameters())?;
        Ok(Self::Both {
            video_decoder: video_ctx.decoder().video()?,
            video_stream_idx: video_stream.index(),
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

    pub fn new_video_only(video_stream: Stream) -> Result<Self, DecodeError> {
        let video_ctx = Context::from_parameters(video_stream.parameters())?;
        Ok(Self::VideoOnly {
            video_decoder: video_ctx.decoder().video()?,
            video_stream_idx: video_stream.index(),
        })
    }

    /// sends packet to approptiate decoder, otherwise discards it
    pub fn send_packet(&mut self, packet: &Packet) -> Result<(), ffmpeg_next::Error> {
        let packet_stream_idx = packet.stream();
        match self {
            EitherOrBoth::VideoOnly { video_decoder, video_stream_idx } => {
                if packet_stream_idx != *video_stream_idx {
                    return Ok(())
                }
                video_decoder.send_packet(packet)
            },
            EitherOrBoth::AudioOnly { audio_decoder, audio_stream_idx } => {
                if packet_stream_idx != *audio_stream_idx {
                    return Ok(())
                }
                audio_decoder.send_packet(packet)
            },
            EitherOrBoth::Both { video_decoder, video_stream_idx, audio_decoder, audio_stream_idx } => {
                if packet_stream_idx == *video_stream_idx {
                    return video_decoder.send_packet(packet)
                }
                if packet_stream_idx == *audio_stream_idx {
                    return audio_decoder.send_packet(packet)
                }
                return Ok(())
            },
        }
    }

    pub fn try_receive_any_frame(&mut self) -> Result<Either<VideoFrame, AudioFrame>, DecodeError> {
        match self {
            EitherOrBoth::VideoOnly { video_decoder, video_stream_idx } => self.try_receive_video_frame().map(Either::Left),
            EitherOrBoth::AudioOnly { audio_decoder, audio_stream_idx } => self.try_receive_audio_frame().map(Either::Right),
            EitherOrBoth::Both { video_decoder, video_stream_idx, audio_decoder, audio_stream_idx } => {
                todo!()
            },
        }
    }

    pub fn try_receive_video_frame(&mut self) -> Result<VideoFrame, DecodeError> {
        match self {
            EitherOrBoth::VideoOnly { video_decoder, video_stream_idx } => {
                image_from_decoder(video_decoder, todo!(), todo!())
            },
            EitherOrBoth::AudioOnly { audio_decoder, audio_stream_idx } => {
                Err(DecodeError::NoSuchStream("video"))
            },
            EitherOrBoth::Both { video_decoder, video_stream_idx, audio_decoder, audio_stream_idx } => {
                image_from_decoder(video_decoder, todo!(), todo!())
            },
        }
    }

    pub fn try_receive_audio_frame(&mut self) -> Result<AudioFrame, DecodeError> {
        match self {
            EitherOrBoth::VideoOnly { video_decoder, video_stream_idx } => {
                Err(DecodeError::NoSuchStream("video"))
            },
            EitherOrBoth::AudioOnly { audio_decoder, audio_stream_idx } => {
                audio_from_decoder(audio_decoder)
            },
            EitherOrBoth::Both { video_decoder, video_stream_idx, audio_decoder, audio_stream_idx } => {
                audio_from_decoder(audio_decoder)
            },
        }
    }
}

pub struct Decoder {
    decoders: EitherOrBoth,
    target_height: u32,
}

impl Decoder {
    pub fn new(
        video_stream: Stream,
        audio_stream: Stream,
        target_height: u32,
    ) -> Result<Self, DecodeError> {

        Ok(Self {
            decoders: EitherOrBoth::new_both(video_stream, audio_stream)?,
            target_height,
        })
    }

    pub fn into_frame_iter(self, input: Input) -> DecodeIter {
        DecodeIter {
            input,
            decoders: self.decoders,
            target_height: self.target_height,
        }
    }
}
