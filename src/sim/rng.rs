//! The single seeded RNG. Implemented here (xoshiro256** seeded by SplitMix64)
//! rather than pulled from a crate so the stream can never change under us with
//! a dependency upgrade: changing it changes every Video.

#[derive(Debug, Clone)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn from_seed(seed: u64) -> Self {
        let mut sm = seed;
        let mut next = || {
            sm = sm.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = sm;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        Rng {
            s: [next(), next(), next(), next()],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform in `[lo, hi)`.
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }
}
