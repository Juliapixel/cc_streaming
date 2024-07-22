use image::{Pixel, Rgb};

use super::range::{Channel, GreatestRange, Ranges};

fn max_range_from_slice(slice: &[Rgb<u8>]) -> GreatestRange {
    if let Some(first) = slice.first() {
        let mut ranges = Ranges::new(*first);
        for i in slice.iter() {
            ranges.update(*i);
        }
        ranges.into()
    } else {
        GreatestRange::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelBucket {
    start: usize,
    end: usize,
    max_range: Option<GreatestRange>,
}

impl PixelBucket {
    pub fn new(start: usize, end: usize, slice: &[Rgb<u8>]) -> Self {
        Self {
            start,
            end,
            max_range: Some(max_range_from_slice(&slice[start..end])),
        }
    }

    pub fn max_range(&mut self, slice: &[Rgb<u8>]) -> GreatestRange {
        self.max_range = Some(
            self.max_range
                .unwrap_or(max_range_from_slice(&slice[self.start..self.end])),
        );
        self.max_range.unwrap()
    }

    pub fn sort_by_greatest_range(&mut self, slice: &mut [Rgb<u8>]) {
        match self.max_range(slice).channel {
            Channel::Red => slice[self.start..self.end].sort_unstable_by_key(|elem| elem.0[0]),
            Channel::Green => slice[self.start..self.end].sort_unstable_by_key(|elem| elem.0[1]),
            Channel::Blue => slice[self.start..self.end].sort_unstable_by_key(|elem| elem.0[2]),
        }
    }

    pub fn split_at_median(self) -> (Self, Self) {
        let midpoint = self.start + ((self.end - self.start) / 2);
        (
            Self {
                start: self.start,
                end: midpoint,
                max_range: None,
            },
            Self {
                start: midpoint,
                end: self.end,
                max_range: None,
            },
        )
    }

    pub fn average_colors(&self, slice: &[Rgb<u8>]) -> Rgb<u8> {
        let mut avg = slice[self.start..self.end]
            .first()
            .copied()
            .unwrap_or(Rgb([0; 3]));
        for i in &slice[self.start..self.end] {
            avg.blend(i)
        }
        avg
    }
}
