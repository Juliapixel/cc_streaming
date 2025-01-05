use image::Rgb;

#[derive(Debug)]
#[repr(align(8))]
pub struct Ranges {
    r_min: u8,
    r_max: u8,
    g_min: u8,
    g_max: u8,
    b_min: u8,
    b_max: u8,
}

impl Ranges {
    pub fn new(pixel: Rgb<u8>) -> Self {
        Self {
            r_min: pixel.0[0],
            r_max: pixel.0[0],
            g_min: pixel.0[1],
            g_max: pixel.0[1],
            b_min: pixel.0[2],
            b_max: pixel.0[2],
        }
    }

    pub fn update(&mut self, new: Rgb<u8>) {
        self.r_min = new.0[0].min(self.r_min);
        self.r_max = new.0[0].max(self.r_max);
        self.g_min = new.0[1].min(self.g_min);
        self.g_max = new.0[1].max(self.g_max);
        self.b_min = new.0[2].min(self.b_min);
        self.b_max = new.0[2].max(self.b_max);
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
        let r_range = value.r_max - value.r_min;
        let g_range = value.g_max - value.g_min;
        let b_range = value.b_max - value.b_min;

        let (max_idx, max_range) = [r_range, g_range, b_range]
            .into_iter()
            .enumerate()
            .max_by_key(|i| i.1)
            .unwrap();
        Self {
            range: max_range,
            channel: match max_idx {
                0 => Channel::Red,
                1 => Channel::Green,
                2 => Channel::Blue,
                _ => unreachable!("what?"),
            },
        }
    }
}
