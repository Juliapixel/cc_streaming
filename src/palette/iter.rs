use std::ops::Deref;

use image::{buffer::EnumeratePixels, Rgb};

use super::{from_rgb_to_oklab, BAYER_4X4};

pub struct PaletteIndexIter<'a, T: Deref<Target = [Rgb<u8>]>> {
    pub(super) palette: T,
    pub(super) pixels: EnumeratePixels<'a, Rgb<u8>>,
}

impl<'a, T: Deref<Target = [Rgb<u8>]>> Iterator for PaletteIndexIter<'a, T> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let points: Vec<[f32; 3]> = self
            .palette
            .iter()
            .map(|p| {
                let okcol = from_rgb_to_oklab(*p);
                [okcol.l, okcol.a, okcol.b]
            })
            .collect();

        let tree = kd_tree::KdIndexTree3::build_by_ordered_float(&points);

        let (x, y, pix) = self.pixels.next()?;
        let okpix = from_rgb_to_oklab(*pix);
        let nearest = tree
            .nearest(&[
                okpix.l + (BAYER_4X4[y as usize % 4][x as usize % 4] / self.palette.len() as f32),
                okpix.a + (BAYER_4X4[y as usize % 4][x as usize % 4] / self.palette.len() as f32),
                okpix.b + (BAYER_4X4[y as usize % 4][x as usize % 4] / self.palette.len() as f32),
            ])
            .unwrap();

        Some(*nearest.item)
    }
}
