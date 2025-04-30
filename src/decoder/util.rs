use scuffle_ffmpeg::decoder;

use crate::{
    dimensions::ResolutionHint,
    frame::{AudioFrame, VideoFrame},
};

use super::DecodeError;

#[inline]
pub fn image_from_decoder(
    decoder: &mut decoder::VideoDecoder,
    resolution_hint: &ResolutionHint,
) -> Result<Option<VideoFrame>, DecodeError> {
    match decoder.receive_frame() {
        Ok(Some(decoded)) => {
            let (width, height) =
                resolution_hint.get_target_res(decoded.width() as u32, decoded.height() as u32);
            Ok(Some(VideoFrame::from_ffmpeg(
                &decoded,
                decoder.time_base().unwrap().into(),
                width,
                height,
            )?))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e)?,
    }
}

#[inline]
pub fn audio_from_decoder(
    decoder: &mut decoder::AudioDecoder,
) -> Result<Option<AudioFrame>, DecodeError> {
    match decoder.receive_frame() {
        Ok(Some(decoded)) => Ok(Some(AudioFrame::from_ffmpeg(
            &decoded,
            decoder.time_base().unwrap().into(),
        )?)),
        Ok(None) => Ok(None),
        Err(e) => Err(e)?,
    }
}
