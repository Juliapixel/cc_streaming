//! basically just helper types to store ffmpeg frames which also include extra
//! info

use std::ops::{Deref, DerefMut};

use image::{GenericImageView, Rgb, RgbImage};
use scuffle_ffmpeg::{
    AVSampleFormat,
    frame::{AudioChannelLayout, AudioFrame as Audio, VideoFrame as Video},
    resampler::Resampler,
    scaler::VideoScaler,
};

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
        let mut converter = VideoScaler::new(
            decoded.width() as i32,
            decoded.height() as i32,
            decoded.format(),
            decoded.width() as i32,
            decoded.height() as i32,
            scuffle_ffmpeg::AVPixelFormat::Rgb24,
        )?;

        let converted = converter.process(decoded)?;

        let samples = {
            let mut tmp_samples = Vec::new();
            let data = converted.data(0).unwrap();
            for row in 0..data.height() {
                tmp_samples.extend_from_slice(data.get_row(row as usize).unwrap());
            }
            tmp_samples
        };

        let flat_image = image::FlatSamples {
            layout: image::flat::SampleLayout {
                channels: 3,
                channel_stride: 1,
                width: converted.width() as u32,
                width_stride: 3,
                height: converted.height() as u32,
                height_stride: converted.linesize(0).unwrap() as usize,
            },
            samples,
            color_hint: None,
        };
        let image = {
            match flat_image.try_into_buffer::<Rgb<u8>>() {
                Ok(ok) => ok,
                Err((_, samples)) => {
                    let view = samples
                        .as_view::<Rgb<u8>>()
                        .map_err(|_| DecodeError::ImageError)?;
                    let mut new_buf =
                        image::ImageBuffer::<Rgb<u8>, _>::new(view.width(), view.height());
                    view.pixels()
                        .for_each(|(x, y, pix)| new_buf.put_pixel(x, y, pix));
                    new_buf
                }
            }
        };

        let mut target_image = fast_image_resize::images::Image::new(
            width,
            height,
            fast_image_resize::PixelType::U8x3,
        );
        let mut resizer = fast_image_resize::Resizer::new();
        resizer
            .resize(
                &image::DynamicImage::from(image),
                &mut target_image,
                &fast_image_resize::ResizeOptions {
                    algorithm: fast_image_resize::ResizeAlg::Convolution(
                        fast_image_resize::FilterType::Hamming,
                    ),
                    ..Default::default()
                },
            )
            .unwrap();
        let image = image::RgbImage::from_vec(width, height, target_image.into_vec()).unwrap();

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
        let mut resampler = Resampler::new(
            decoded.channel_layout(),
            decoded.format(),
            decoded.sample_rate(),
            AudioChannelLayout::new(1)?,
            AVSampleFormat::Fltp,
            48000,
        )?;

        let mut resampled = resampler.process(decoded)?;
        let nb_samples = resampled.nb_samples() as usize;

        let buf = resampled
            .data_mut(0)
            .expect("no data[0] present in AudioFrame... wtf?");

        let samples = bytemuck::try_cast_slice_mut::<u8, f32>(buf)
            .map_err(|_| DecodeError::AudioFrameLength)?
            .split_at(nb_samples)
            .0;

        log::debug!("audio frame received: {nb_samples} samples");

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
