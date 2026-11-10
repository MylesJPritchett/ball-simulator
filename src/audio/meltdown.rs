//! The Meltdown's sound: crackle under the Play-out, the finished soundtrack
//! stuttering like a stuck buffer, then a hard cut to silence at the freeze.
//!
//! Like the picture, the breakdown is staged from the Seed on its own RNG
//! stream (ADR 0001): the same record always crackles and stutters the same.

use super::{SAMPLES_PER_SUBSTEP, SAMPLE_RATE};
use crate::sim::rng::Rng;
use crate::sim::Ending;

/// XORed into the Seed for the crackle's and stutter's RNG streams.
const CRACKLE_STREAM: u64 = 0x6372_6163_6b6c_6521;
const STUTTER_STREAM: u64 = 0x7374_7574_7465_7221;
/// Crackles per second as the Play-out starts and as it ends.
const CRACKLES_PER_SECOND: (f64, f64) = (40.0, 240.0);
/// Peak level of one crackle.
const CRACKLE_GAIN: f64 = 0.5;
/// How long one crackle rings, in seconds (the longest).
const CRACKLE_SECONDS: f64 = 0.004;
/// Length range of the buffer that repeats in one stutter, in seconds.
const STUTTER_BUFFER_SECONDS: (f64, f64) = (0.02, 0.09);
/// Most times one buffer repeats, at the end of the Play-out.
const MAX_REPEATS: f64 = 6.0;

fn sample_of(substep: u64) -> usize {
    (substep * SAMPLES_PER_SUBSTEP) as usize
}

/// Adds crackle to the dry mix over the Play-out: short bursts of decaying
/// noise, denser as the Play-out runs.
pub fn crackle(dry: &mut [f64], ending: &Ending, seed: u64) {
    let (start, freeze) = (
        sample_of(ending.start_substep),
        sample_of(ending.freeze_substep),
    );
    let freeze = freeze.min(dry.len());
    let mut rng = Rng::from_seed(seed ^ CRACKLE_STREAM);
    let mut at = start as f64;
    while (at as usize) < freeze {
        let t = (at - start as f64) / (freeze - start) as f64;
        let rate = CRACKLES_PER_SECOND.0 + (CRACKLES_PER_SECOND.1 - CRACKLES_PER_SECOND.0) * t;
        let gain = CRACKLE_GAIN * rng.range_f64(0.3, 1.0);
        let len = (CRACKLE_SECONDS * SAMPLE_RATE as f64 * rng.range_f64(0.2, 1.0)) as usize;
        let from = at as usize;
        for (i, sample) in dry[from..(from + len).min(freeze)].iter_mut().enumerate() {
            let decay = 1.0 - i as f64 / len as f64;
            *sample += gain * decay * decay * rng.range_f64(-1.0, 1.0);
        }
        // Exponential gaps: crackle arrives at random, `rate` per second.
        at += -(1.0 - rng.next_f64()).ln() / rate * SAMPLE_RATE as f64;
    }
}

/// Stutters the finished soundtrack over the Play-out (a short buffer
/// repeats, more times as it goes on) and cuts it to silence at the freeze.
pub fn stutter_and_cut(out: &mut [f32], ending: &Ending, seed: u64) {
    let (start, freeze) = (
        sample_of(ending.start_substep),
        sample_of(ending.freeze_substep),
    );
    let freeze = freeze.min(out.len());
    let mut rng = Rng::from_seed(seed ^ STUTTER_STREAM);
    let mut at = start;
    while at < freeze {
        let t = (at - start) as f64 / (freeze - start) as f64;
        let (lo, hi) = STUTTER_BUFFER_SECONDS;
        let len = (rng.range_f64(lo, hi) * SAMPLE_RATE as f64) as usize;
        let repeats = 1 + (MAX_REPEATS * t * rng.range_f64(0.5, 1.0)).round() as usize;
        // The buffer plays, then repeats in place of what came after it.
        let buffer = out[at..(at + len).min(freeze)].to_vec();
        at += buffer.len();
        for _ in 1..repeats {
            let end = (at + buffer.len()).min(freeze);
            out[at..end].copy_from_slice(&buffer[..end - at]);
            at = end;
        }
    }
    out[freeze..].fill(0.0);
}
