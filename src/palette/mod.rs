use bucket::PixelBucket;
use image::Rgb;
use palette::{color_difference::EuclideanDistance, FromColor, Oklab, Srgb};

mod bucket;
mod range;

fn from_rgb_to_oklab(rgb: Rgb<u8>) -> Oklab {
    palette::oklab::Oklab::from_color(palette::Srgb::new(
        rgb.0[0] as f32 / 255.0,
        rgb.0[1] as f32 / 255.0,
        rgb.0[2] as f32 / 255.0,
    ))
}

fn from_ok_lab_to_rgb(oklab: Oklab) -> Rgb<u8> {
    let srgb = Srgb::from_color(oklab);
    Rgb([
        (srgb.red * 255.0).clamp(0.0, 255.0) as u8,
        (srgb.green * 255.0).clamp(0.0, 255.0) as u8,
        (srgb.blue * 255.0).clamp(0.0, 255.0) as u8,
    ])
}

#[derive(Debug, thiserror::Error)]
pub enum PaletteError {}

pub struct Palette {
    palette: Vec<Rgb<u8>>,
}

impl Palette {
    pub fn new(size: usize, image: &image::RgbImage) -> Self {
        let mut pixels: Vec<Rgb<u8>> =
            Vec::with_capacity(image.width() as usize * image.height() as usize);
        pixels.extend(image.pixels());

        let mut buckets: Vec<PixelBucket> = vec![PixelBucket::new(0, pixels.len(), &pixels)];
        while buckets.len() < size {
            let mut biggest_idx = 0;
            for i in 0..buckets.len() {
                if buckets[i].max_range(&pixels).range
                    > buckets[biggest_idx].max_range(&pixels).range
                {
                    biggest_idx = i;
                }
            }
            let mut biggest = buckets[biggest_idx];
            biggest.sort_by_greatest_range(&mut pixels);

            let (left, right) = biggest.split_at_median();
            buckets.retain(|b| *b != biggest);
            buckets.push(left);
            buckets.push(right);
        }

        let mut palette: Vec<Rgb<u8>> = Vec::with_capacity(size);
        for i in buckets {
            palette.push(i.average_colors(&pixels))
        }

        Self { palette }
    }

    pub fn apply(&self, img: &mut image::RgbImage) {
        for pix in img.pixels_mut() {
            let mut closest = from_rgb_to_oklab(self.palette[0]);
            let mut min_squared_dist = closest.distance_squared(from_rgb_to_oklab(*pix)).abs();
            let okpix = from_rgb_to_oklab(*pix);
            for col in self.iter() {
                let col = from_rgb_to_oklab(*col);
                let squared_dist = okpix.distance_squared(col).abs();
                if squared_dist < min_squared_dist {
                    min_squared_dist = squared_dist;
                    closest = col;
                }
            }
            *pix = from_ok_lab_to_rgb(closest);
        }
    }

    pub fn palette(&self) -> &[Rgb<u8>] {
        &self.palette
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Rgb<u8>> {
        self.palette.iter()
    }
}

impl IntoIterator for Palette {
    type Item = Rgb<u8>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.palette.into_iter()
    }
}

impl<'a> IntoIterator for &'a Palette {
    type Item = &'a Rgb<u8>;

    type IntoIter = core::slice::Iter<'a, Rgb<u8>>;

    fn into_iter(self) -> Self::IntoIter {
        self.palette.iter()
    }
}
