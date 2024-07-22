use std::ops::RangeInclusive;

use image::Rgb;

#[derive(Debug)]
pub struct Ranges {
    r: RangeInclusive<u8>,
    g: RangeInclusive<u8>,
    b: RangeInclusive<u8>,
}

impl Ranges {
    pub fn new(pixel: Rgb<u8>) -> Self {
        Self {
            r: pixel.0[0]..=pixel.0[0],
            g: pixel.0[1]..=pixel.0[1],
            b: pixel.0[2]..=pixel.0[2],
        }
    }

    pub fn update(&mut self, new: Rgb<u8>) {
        self.r = new.0[0].min(*self.r.start())..=new.0[0].max(*self.r.end());
        self.g = new.0[1].min(*self.g.start())..=new.0[1].max(*self.g.end());
        self.b = new.0[2].min(*self.b.start())..=new.0[2].max(*self.b.end());
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GreatestRange {
    pub range: u8,
    pub channel: Channel,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    #[default]
    Red,
    Green,
    Blue,
}

impl From<Ranges> for GreatestRange {
    fn from(value: Ranges) -> Self {
        let r_range = value.r.end() - value.r.start();
        let g_range = value.g.end() - value.g.start();
        let b_range = value.b.end() - value.b.start();

        let max_range = *[r_range, g_range, b_range].iter().max().unwrap();
        if max_range == r_range {
            Self {
                range: max_range,
                channel: Channel::Red,
            }
        } else if max_range == g_range {
            Self {
                range: max_range,
                channel: Channel::Green,
            }
        } else if max_range == b_range {
            Self {
                range: max_range,
                channel: Channel::Blue,
            }
        } else {
            unreachable!("what?");
        }
    }
}
