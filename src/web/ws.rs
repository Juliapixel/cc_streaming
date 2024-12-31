use fast_image_resize::PixelType;
use serde::Serialize;

use crate::palette::Palette;

#[derive(Debug, Clone, Serialize)]
pub struct StreamVideoFrame {
    pub palette: Vec<[u8; 3]>,
    pub rows: Vec<String>,
}

impl StreamVideoFrame {
    pub fn from_image_scaled(image: image::RgbImage, width: u32, height: u32) -> Self {
        let mut target_image =
            fast_image_resize::images::Image::new(width, height, PixelType::U8x3);

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

        (&image).into()
    }

    pub fn from_image(image: &image::RgbImage) -> Self {
        image.into()
    }
}

impl From<&image::RgbImage> for StreamVideoFrame {
    fn from(image: &image::RgbImage) -> Self {
        let palette = Palette::new(16, image);

        let mut lines: Vec<String> = (0..image.height())
            .map(|_| String::with_capacity(image.width() as usize))
            .collect();

        for (i, pal_idx) in palette.index_iter(image).enumerate() {
            let line = i / image.width() as usize;
            lines[line].push(char::from_digit(pal_idx as u32, 16).unwrap());
        }

        StreamVideoFrame {
            palette: palette
                .into_iter()
                .map(|pix| [pix.0[0], pix.0[1], pix.0[2]])
                .collect(),
            rows: lines,
        }
    }
}

impl From<image::RgbImage> for StreamVideoFrame {
    fn from(image: image::RgbImage) -> Self {
        let palette = Palette::new(16, &image);

        let mut lines: Vec<String> = (0..image.height())
            .map(|_| String::with_capacity(image.width() as usize))
            .collect();

        for (i, pal_idx) in palette.index_iter(&image).enumerate() {
            let line = i / image.width() as usize;
            lines[line].push(char::from_digit(pal_idx as u32, 16).unwrap());
        }

        StreamVideoFrame {
            palette: palette
                .into_iter()
                .map(|pix| [pix.0[0], pix.0[1], pix.0[2]])
                .collect(),
            rows: lines,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamAudioFrame {
    pub samples: Vec<u8>,
}
