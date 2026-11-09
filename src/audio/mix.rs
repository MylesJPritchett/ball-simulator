//! Final mix processing: a light reverb, then a limiter that guarantees the
//! output never exceeds [`CEILING`].

use super::SAMPLE_RATE;

/// Peak level the limiter holds the mix under (just below full scale).
pub const CEILING: f64 = 0.95;
const WET: f64 = 0.16;
const LOOKAHEAD_SECONDS: f64 = 0.003;
const RELEASE_SECONDS: f64 = 0.2;

/// Small Schroeder reverb (four damped combs into two allpasses), mixed in
/// lightly. Mono in, mono out, same length.
pub fn reverb(dry: &[f64]) -> Vec<f64> {
    let ms = |m: f64| ((m / 1000.0) * SAMPLE_RATE as f64) as usize;
    let mut combs: Vec<Comb> = [29.7, 37.1, 41.1, 43.7]
        .into_iter()
        .map(|d| Comb::new(ms(d), 0.8, 0.3))
        .collect();
    let mut allpasses = [Allpass::new(ms(5.0), 0.7), Allpass::new(ms(1.7), 0.7)];
    dry.iter()
        .map(|&x| {
            let mut wet: f64 = combs.iter_mut().map(|c| c.process(x)).sum::<f64>() * 0.25;
            for ap in &mut allpasses {
                wet = ap.process(wet);
            }
            x + WET * wet
        })
        .collect()
}

struct Comb {
    buf: Vec<f64>,
    pos: usize,
    feedback: f64,
    damp: f64,
    filtered: f64,
}

impl Comb {
    fn new(len: usize, feedback: f64, damp: f64) -> Self {
        Comb {
            buf: vec![0.0; len.max(1)],
            pos: 0,
            feedback,
            damp,
            filtered: 0.0,
        }
    }

    fn process(&mut self, x: f64) -> f64 {
        let y = self.buf[self.pos];
        self.filtered = y * (1.0 - self.damp) + self.filtered * self.damp;
        self.buf[self.pos] = x + self.filtered * self.feedback;
        self.pos = (self.pos + 1) % self.buf.len();
        y
    }
}

struct Allpass {
    buf: Vec<f64>,
    pos: usize,
    gain: f64,
}

impl Allpass {
    fn new(len: usize, gain: f64) -> Self {
        Allpass {
            buf: vec![0.0; len.max(1)],
            pos: 0,
            gain,
        }
    }

    fn process(&mut self, x: f64) -> f64 {
        let delayed = self.buf[self.pos];
        let v = x + self.gain * delayed;
        self.buf[self.pos] = v;
        self.pos = (self.pos + 1) % self.buf.len();
        delayed - self.gain * v
    }
}

/// Offline look-ahead peak limiter. The gain at each sample is never above
/// what that sample needs to stay under [`CEILING`]; it ramps down ahead of
/// peaks and recovers smoothly after them.
pub fn limit(mix: &[f64]) -> Vec<f32> {
    let n = mix.len();
    let needed: Vec<f64> = mix
        .iter()
        .map(|s| {
            let a = s.abs();
            if a > CEILING {
                CEILING / a
            } else {
                1.0
            }
        })
        .collect();
    let mut gain = needed.clone();
    // Backward pass: ramp down linearly over the look-ahead before each peak.
    let attack_step = 1.0 / (LOOKAHEAD_SECONDS * SAMPLE_RATE as f64);
    for i in (0..n.saturating_sub(1)).rev() {
        gain[i] = gain[i].min(gain[i + 1] + attack_step);
    }
    // Forward pass: recover exponentially towards unity gain.
    let release = (-1.0 / (RELEASE_SECONDS * SAMPLE_RATE as f64)).exp();
    for i in 1..n {
        let recovered = 1.0 - (1.0 - gain[i - 1]) * release;
        gain[i] = gain[i].min(recovered);
    }
    mix.iter()
        .zip(&gain)
        .map(|(s, g)| (s * g).clamp(-CEILING, CEILING) as f32)
        .collect()
}
