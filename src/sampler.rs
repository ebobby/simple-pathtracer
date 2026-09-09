//! Padded 2D Sobol sampler with hash-based Owen scrambling.
//!
//! Every random decision along a path is a 2D "pair" drawn from the 2D Sobol
//! sequence. Each pair slot is scrambled and index-shuffled with its own seed
//! (hashed from the pixel seed and the slot), so pairs are decorrelated while
//! every pair keeps the sequence's stratification. This is the scheme from
//! Burley, "Practical Hash-based Owen Scrambling" (JCGT 2020), as used in
//! Cycles. The GPU shader mirrors these functions.

/// Point `index` of the plain 2D Sobol sequence, as 32-bit fixed point.
///
/// The first dimension is the bit reversal of the index. The second
/// dimension's generator matrix is Pascal's triangle mod 2 (rows 1, 11, 101,
/// 1111, ...), whose product with a vector is the Sierpinski butterfly: five
/// masked shift-xor steps applied to the reversed index. A test checks this
/// against the direction-number recurrence.
#[inline]
pub fn sobol_2d(index: u32) -> (u32, u32) {
    let x = index.reverse_bits();
    let mut y = x;
    y ^= (y & 0x5555_5555) << 1;
    y ^= (y & 0x3333_3333) << 2;
    y ^= (y & 0x0f0f_0f0f) << 4;
    y ^= (y & 0x00ff_00ff) << 8;
    y ^= (y & 0x0000_ffff) << 16;
    (x, y)
}

/// Second Sobol dimension from the direction-number recurrence
/// (`m_1 = 1`, `m_k = 2 m_(k-1) ^ m_(k-1)`), kept as the reference for tests.
#[cfg(test)]
pub fn sobol_dim1_reference(index: u32) -> u32 {
    let mut y = 0u32;
    let mut m: u32 = 1;
    let mut bits = index;
    let mut k = 0;
    while bits != 0 {
        if bits & 1 == 1 {
            y ^= m << (31 - k);
        }
        m = (m << 1) ^ m;
        bits >>= 1;
        k += 1;
    }
    y
}

/// Integer hash (lowbias32).
#[inline]
pub fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

#[inline]
fn hash_combine(seed: u32, value: u32) -> u32 {
    hash(seed ^ value.wrapping_mul(0x9e37_79b9))
}

/// Laine-Karras style permutation: each output bit depends only on input
/// bits at or below it, which is what makes the scramble below an Owen
/// scramble. Every step is a bijection: the add carries upwards only, and
/// `x ^= x * c` with even `c` can be inverted bit by bit from the bottom.
#[inline]
fn laine_karras_permutation(mut x: u32, seed: u32) -> u32 {
    x = x.wrapping_add(seed);
    x ^= x.wrapping_mul(0x6c50_b47c);
    x ^= x.wrapping_mul(0xb82f_1e52);
    x ^= x.wrapping_mul(0xc7af_e784);
    x ^= x.wrapping_mul(0x8d22_f6e6);
    x
}

/// Owen scramble of a 32-bit fixed-point value.
#[inline]
pub fn nested_uniform_scramble(x: u32, seed: u32) -> u32 {
    laine_karras_permutation(x.reverse_bits(), seed).reverse_bits()
}

/// Sample generator for one path: a pixel seed and the sample's index.
#[derive(Clone, Copy, Debug)]
pub struct Sampler {
    pixel_seed: u32,
    index: u32,
}

impl Sampler {
    pub fn new(pixel_seed: u32, index: u32) -> Self {
        Self { pixel_seed, index }
    }

    /// The 2D sample for pair `slot`, both coordinates in [0, 1).
    #[inline]
    pub fn get_2d(&self, slot: u32) -> (f64, f64) {
        // The index shuffle and the two coordinate scrambles need
        // independent seeds; deriving them by xoring constants into one hash
        // measurably raised noise on the uneven-lights bench.
        let seed = hash_combine(self.pixel_seed, slot);
        let index = nested_uniform_scramble(self.index, hash_combine(seed, 0));
        let (x, y) = sobol_2d(index);
        let x = nested_uniform_scramble(x, hash_combine(seed, 1));
        let y = nested_uniform_scramble(y, hash_combine(seed, 2));
        (f64::from(x) / 4294967296.0, f64::from(y) / 4294967296.0)
    }
}

/// Pair slot layout along a path.
pub const SLOT_PIXEL: u32 = 0;

/// First slot of bounce `bounce` (0-based). Each bounce uses four slots:
/// BSDF direction, light sample, (light selection / lobe choice / Fresnel,
/// Russian roulette), and a secondary BSDF direction.
#[inline]
pub fn bounce_slot(bounce: u32) -> u32 {
    1 + 4 * bounce
}
