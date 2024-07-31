use ffmpeg_next::{
    decoder,
    frame::{Audio, Video},
};

use crate::frame::{AudioFrame, VideoFrame};

use super::DecodeError;

pub fn image_from_decoder(
    decoder: &mut decoder::Video,
    width: u32,
    height: u32,
) -> Result<VideoFrame, DecodeError> {
    let mut decoded = Video::empty();
    match decoder.receive_frame(&mut decoded) {
        Ok(_) => Ok(VideoFrame::from_ffmpeg(
            &decoded,
            decoder.time_base().into(),
            width,
            height,
        )?),
        Err(e) => Err(e)?,
    }
}

pub fn audio_from_decoder(decoder: &mut decoder::Audio) -> Result<AudioFrame, DecodeError> {
    let mut decoded = Audio::empty();
    match decoder.receive_frame(&mut decoded) {
        Ok(_) => Ok(AudioFrame::from_ffmpeg(
            &decoded,
            decoder.time_base().into(),
        )?),
        Err(e) => Err(e)?,
    }
}
