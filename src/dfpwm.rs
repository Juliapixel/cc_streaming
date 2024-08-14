use bitvec::{order::Msb0, vec::BitVec};

const PREC: i32 = 10;

#[derive(Debug, Default, Clone, Copy)]
pub struct DfpwmEncoder {
    charge: i32,
    strength: i32,
    previous_bit: bool,
}

impl DfpwmEncoder {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn encode(&mut self, samples: impl IntoIterator<Item = f32>) -> Vec<u8> {
        let iter = samples.into_iter();
        let size_hint = iter.size_hint();
        let mut out = BitVec::<u8, Msb0>::with_capacity(size_hint.1.unwrap_or(size_hint.0));

        for sample in iter {
            let level = (sample * 127.0).round().clamp(-128.0, 127.0) as i32;

            let current_bit = level > self.charge || (level == self.charge && self.charge == 127);
            let target = if current_bit { 127 } else { -128 };

            let mut next_charge = self.charge
                + ((self.strength * (target - self.charge) + (1 << (PREC - 1))) >> PREC);
            if next_charge == self.charge && next_charge != target {
                next_charge += if current_bit { 1 } else { -1 };
            }

            let z = if current_bit == self.previous_bit {
                1 << PREC
            } else {
                0
            };
            let mut next_strength = self.strength;
            if self.strength != z {
                next_strength += if current_bit == self.previous_bit {
                    1
                } else {
                    -1
                };
            }
            if next_strength < 2 << (PREC - 8) {
                next_strength = 2 << (PREC - 8);
            }

            self.charge = next_charge;
            self.strength = next_strength;
            self.previous_bit = current_bit;

            out.push(current_bit);
        }

        out.into()
    }
}
