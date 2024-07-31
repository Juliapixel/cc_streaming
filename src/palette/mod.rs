use bucket::PixelBucket;
use image::Rgb;
use iter::PaletteIndexIter;
use once_cell::sync::Lazy;
use palette::{FromColor, Oklab, Srgb};

mod bucket;
pub mod iter;
mod range;

// idk why tf this happens so there u have it ig
#[allow(clippy::eq_op)]
const BAYER_4X4: [[f32; 4]; 4] = [
    [
        (00.0 / 16.0) - 0.5,
        (12.0 / 16.0) - 0.5,
        (03.0 / 16.0) - 0.5,
        (15.0 / 16.0) - 0.5,
    ],
    [
        (08.0 / 16.0) - 0.5,
        (04.0 / 16.0) - 0.5,
        (11.0 / 16.0) - 0.5,
        (07.0 / 16.0) - 0.5,
    ],
    [
        (02.0 / 16.0) - 0.5,
        (14.0 / 16.0) - 0.5,
        (01.0 / 16.0) - 0.5,
        (13.0 / 16.0) - 0.5,
    ],
    [
        (10.0 / 16.0) - 0.5,
        (06.0 / 16.0) - 0.5,
        (09.0 / 16.0) - 0.5,
        (05.0 / 16.0) - 0.5,
    ],
];

static OKLAB_LUT: Lazy<Vec<Oklab>> = Lazy::new(|| {
    let mut lut = Vec::with_capacity(256 * 256 * 256);
    for b in 0..=255u8 {
        for g in 0..=255u8 {
            for r in 0..=255u8 {
                lut.push(Oklab::from_color(Srgb::new(r, g, b).into_format::<f32>()))
            }
        }
    }
    lut
});

#[inline(always)]
fn from_rgb_to_oklab(rgb: Rgb<u8>) -> Oklab {
    OKLAB_LUT[(rgb.0[0] as usize * 256 * 256) + (rgb.0[1] as usize * 256) + rgb.0[2] as usize]
}

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
            buckets.swap_remove(biggest_idx);
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
        let points: Vec<[f32; 3]> = self
            .palette
            .iter()
            .map(|p| {
                let okcol = from_rgb_to_oklab(*p);
                [okcol.l, okcol.a, okcol.b]
            })
            .collect();

        let tree = kd_tree::KdIndexTree3::build_by_ordered_float(&points);

        // let mut tree: kdtree::KdTree<_, Rgb<u8>, [f32; 3]> = kdtree::KdTree::new(3);

        // self.palette.iter().for_each(|p| {
        //     let okpal = from_rgb_to_oklab(*p);
        //     tree.add([okpal.l, okpal.a, okpal.b], *p).unwrap();
        // });

        for (x, y, pix) in img.enumerate_pixels_mut() {
            let okpix = from_rgb_to_oklab(*pix);
            let nearest = tree
                .nearest(&[
                    okpix.l
                        + (BAYER_4X4[y as usize % 4][x as usize % 4] / self.palette.len() as f32),
                    okpix.a
                        + (BAYER_4X4[y as usize % 4][x as usize % 4] / self.palette.len() as f32),
                    okpix.b
                        + (BAYER_4X4[y as usize % 4][x as usize % 4] / self.palette.len() as f32),
                ])
                .unwrap();
            *pix = self.palette[*nearest.item];
        }
    }

    pub fn palette(&self) -> &[Rgb<u8>] {
        &self.palette
    }

    pub fn index_iter<'a>(&self, img: &'a image::RgbImage) -> PaletteIndexIter<'a, &[Rgb<u8>]> {
        PaletteIndexIter {
            palette: &self.palette,
            pixels: img.enumerate_pixels(),
        }
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
