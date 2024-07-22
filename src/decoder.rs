use ffmpeg_next::{
    codec::Context,
    decoder,
    format::context::Input,
    frame::{Audio, Video},
    software::scaling::Flags,
    Stream,
};
use image::RgbImage;

pub struct Decoder {
    video_decoder: decoder::Video,
    video_stream_idx: usize,
    audio_decoder: decoder::Audio,
    audio_stream_idx: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error(transparent)]
    FfmpegError(#[from] ffmpeg_next::Error),
    #[error("failed to convert frame to image")]
    ImageError,
    #[error("audio frame had a length not divisible by 4")]
    AudioFrameLength,
}

impl Decoder {
    pub fn new(video_stream: Stream, audio_stream: Stream) -> Result<Self, DecodeError> {
        let video_ctx = Context::from_parameters(video_stream.parameters())?;
        let audio_ctx = Context::from_parameters(audio_stream.parameters())?;
        Ok(Self {
            video_decoder: video_ctx.decoder().video()?,
            video_stream_idx: video_stream.index(),
            audio_decoder: audio_ctx.decoder().audio()?,
            audio_stream_idx: audio_stream.index(),
        })
    }

    pub fn decode_all(
        &mut self,
        input: &mut Input,
    ) -> Result<(Vec<RgbImage>, Vec<f32>), DecodeError> {
        let mut video_frames: Vec<image::RgbImage> = Vec::new();
        let mut audio_wave: Vec<f32> = Vec::new();

        let mut push_vid_frames = |decoder: &mut decoder::Video| -> Result<(), DecodeError> {
            let mut decoded = Video::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                let mut scaler = ffmpeg_next::software::scaling::Context::get(
                    decoded.format(),
                    decoded.width(),
                    decoded.height(),
                    ffmpeg_next::format::Pixel::RGB24,
                    (decoded.width() as f32 / decoded.height() as f32 * 144.0).round() as u32,
                    144,
                    Flags::BILINEAR,
                )?;

                let mut converted = Video::empty();
                scaler.run(&decoded, &mut converted)?;

                let image = image::RgbImage::from_raw(
                    converted.width(),
                    converted.height(),
                    converted.data(0).into(),
                )
                .ok_or(DecodeError::ImageError)?;
                video_frames.push(image);
            }
            Ok(())
        };

        let mut push_audio_frames = |decoder: &mut decoder::Audio| -> Result<(), DecodeError> {
            let mut decoded = ffmpeg_next::util::frame::Audio::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                let mut resampler = decoded.resampler(
                    ffmpeg_next::format::Sample::F32(ffmpeg_next::format::sample::Type::Planar),
                    ffmpeg_next::ChannelLayout::MONO,
                    24000,
                )?;
                let mut resampled = Audio::empty();
                resampler.run(&decoded, &mut resampled)?;

                let buf = resampled.data(0);
                if buf.len() % 4 != 0 {
                    return Err(DecodeError::AudioFrameLength);
                }
                // HEEEEEEEEELP
                unsafe {
                    let samples =
                        core::slice::from_raw_parts::<f32>(buf.as_ptr() as _, buf.len() / 4);
                    audio_wave.extend_from_slice(samples);
                }
            }

            Ok(())
        };

        for (stream, packet) in input.packets() {
            if stream.index() == self.video_stream_idx {
                self.video_decoder.send_packet(&packet)?;
                push_vid_frames(&mut self.video_decoder)?;
            }
            if stream.index() == self.audio_stream_idx {
                self.audio_decoder.send_packet(&packet)?;
                push_audio_frames(&mut self.audio_decoder)?;
            }
        }

        push_vid_frames(&mut self.video_decoder)?;
        push_audio_frames(&mut self.audio_decoder)?;
        self.video_decoder.send_eof()?;
        self.audio_decoder.send_eof()?;

        Ok((video_frames, audio_wave))
    }
}
