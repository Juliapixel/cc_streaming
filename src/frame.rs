//! basically just helper types to store ffmpeg frames which also include extra
//! info

use std::ops::{Deref, DerefMut};

use ffmpeg_next::frame::{Audio, Video};
use image::RgbImage;

use crate::decoder::DecodeError;

#[derive(Debug, Clone)]
pub struct VideoFrame {
    timestamp: f64,
    image: RgbImage,
}

impl Deref for VideoFrame {
    type Target = RgbImage;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

impl DerefMut for VideoFrame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.image
    }
}

impl VideoFrame {
    pub fn from_ffmpeg(
        decoded: &Video,
        time_base: f64,
        width: u32,
        height: u32,
    ) -> Result<Self, DecodeError> {
        let mut converter = decoded.converter(ffmpeg_next::format::Pixel::RGB24)?;

        let mut converted = Video::empty();
        converter.run(decoded, &mut converted)?;

        let buf = Vec::from(converted.data(0));
        let image = image::RgbImage::from_raw(converted.width(), converted.height(), buf)
            .ok_or(DecodeError::ImageError)?;

        let image =
            image::imageops::resize(&image, width, height, image::imageops::FilterType::Nearest);

        let ts = decoded.pts().unwrap() as f64 * time_base;

        Ok(VideoFrame {
            timestamp: ts,
            image,
        })
    }

    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }

    pub fn image(&self) -> &RgbImage {
        &self.image
    }
}

#[derive(Debug, Clone)]
pub struct AudioFrame {
    samples: Vec<f32>,
    timestamp: f64,
}

impl Deref for AudioFrame {
    type Target = Vec<f32>;

    fn deref(&self) -> &Self::Target {
        &self.samples
    }
}

impl DerefMut for AudioFrame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.samples
    }
}

impl AudioFrame {
    pub fn from_ffmpeg(decoded: &Audio, time_base: f64) -> Result<Self, DecodeError> {
        let mut resampler = decoded.resampler(
            ffmpeg_next::format::Sample::F32(ffmpeg_next::format::sample::Type::Planar),
            ffmpeg_next::ChannelLayout::MONO,
            44100,
        )?;
        let mut resampled = Audio::empty();
        resampler.run(decoded, &mut resampled)?;

        let buf = resampled.data(0);
        if buf.len() % 4 != 0 {
            return Err(DecodeError::AudioFrameLength);
        }
        // HEEEEEEEEELP
        let samples =
            unsafe { core::slice::from_raw_parts::<f32>(buf.as_ptr() as _, buf.len() / 4) };

        let ts = decoded.pts().unwrap() as f64 * time_base;

        Ok(Self {
            samples: samples.into(),
            timestamp: ts,
        })
    }

    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
}
