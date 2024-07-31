use bitvec::{order::Msb0, vec::BitVec};

const PREC: i32 = 10;

pub struct DfwpmEncoder {}

impl DfwpmEncoder {
    #[inline]
    pub fn encode(samples: &[f32]) -> Vec<u8> {
        let mut charge: i32 = 0;
        let mut strength: i32 = 0;
        let mut previous_bit = false;

        let mut out = BitVec::<u8, Msb0>::with_capacity(samples.len());

        for sample in samples {
            let level = (sample * 127.0).round().clamp(-128.0, 127.0) as i32;

            let current_bit = level > charge || (level == charge && charge == 127);
            let target = if current_bit { 127 } else { -128 };

            let mut next_charge =
                charge + ((strength * (target - charge) + (1 << (PREC - 1))) >> PREC);
            if next_charge == charge && next_charge != target {
                next_charge += if current_bit { 1 } else { -1 };
            }

            let z = if current_bit == previous_bit {
                1 << PREC
            } else {
                0
            };
            let mut next_strength = strength;
            if strength != z {
                next_strength += if current_bit == previous_bit { 1 } else { -1 };
            }
            if next_strength < 2 << (PREC - 8) {
                next_strength = 2 << (PREC - 8);
            }

            charge = next_charge;
            strength = next_strength;
            previous_bit = current_bit;

            out.push(current_bit);
        }

        out.into()
    }
}
