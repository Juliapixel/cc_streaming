use ffmpeg_next::{
    decoder,
    frame::{Audio, Video},
};

use crate::{
    dimensions::ResolutionHint,
    frame::{AudioFrame, VideoFrame},
};

use super::DecodeError;

#[inline]
pub fn image_from_decoder(
    decoder: &mut decoder::Video,
    resolution_hint: &ResolutionHint,
) -> Result<VideoFrame, DecodeError> {
    let mut decoded = Video::empty();
    match decoder.receive_frame(&mut decoded) {
        Ok(_) => {
            let (width, height) = resolution_hint.get_target_res(decoded.width(), decoded.height());
            Ok(VideoFrame::from_ffmpeg(
                &decoded,
                decoder.time_base().into(),
                width,
                height,
            )?)
        }
        Err(e) => Err(e)?,
    }
}

#[inline]
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
