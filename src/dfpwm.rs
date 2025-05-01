use bitvec::{order::Lsb0, vec::BitVec};

const PREC: i32 = 10;

#[derive(Debug, Default, Clone, Copy)]
pub struct DfpwmEncoder {
    /// charge
    q: i32,
    /// strength
    s: i32,
    /// previous bit
    prev_b: bool,
}

impl DfpwmEncoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn encode(&mut self, samples: impl IntoIterator<Item = f32>) -> Vec<u8> {
        let iter = samples.into_iter();
        let size_hint = iter.size_hint();
        let mut out = BitVec::<u8, Lsb0>::with_capacity(size_hint.1.unwrap_or(size_hint.0));

        for sample in iter {
            let sample = (sample * 127.0).floor().clamp(-128.0, 127.0) as i32;

            let curr_b = sample > self.q || (sample == self.q && self.q == 127);

            // target
            let t = if curr_b { 127 } else { -128 };

            let mut next_q = self.q + ((self.s * (t - self.q) + (1 << (PREC - 1))) >> PREC);

            if next_q == self.q && next_q != t {
                next_q += if curr_b { 1 } else { -1 };
            }

            let z = if curr_b == self.prev_b {
                (1 << PREC) - 1
            } else {
                0
            };

            let mut next_s = self.s;
            if next_s != z {
                next_s += if z != 0 { 1 } else { -1 };
            }
            if PREC > 8 && next_s < (1 << (PREC - 7)) {
                next_s = 1 << (PREC - 7);
            }

            self.q = next_q;
            self.s = next_s;
            self.prev_b = curr_b;

            out.push(curr_b);
        }

        out.truncate(out.len() - out.len() % 8);

        out.into()
    }
}

/// compares encoding to `https://music.madefor.cc/`
#[test]
fn squiddev_encode() {
    let mut encoder = DfpwmEncoder::new();
    let buf: [f32; 256] = core::array::from_fn(|i| (i as f32).sin());
    let encoded = encoder.encode(buf);

    assert_eq!(
        encoded,
        [
  142, 227,  56,  28, 199, 241,  56, 142,
  195, 113,  28, 143, 227,  56,  28, 199,
  241,  56, 142, 227, 113,  28, 135, 227,
   56,  30, 199, 113,  56, 142, 227, 113
]
    )
}
