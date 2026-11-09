//! Audio: synthesizes the whole soundtrack offline from the simulation record,
//! sample-accurately from Boundary Hit substep timestamps, and writes it as WAV.

mod wav;

pub use wav::write_wav;

use crate::sim::{SimRecord, SUBSTEP_HZ};

pub const SAMPLE_RATE: u32 = 48_000;
/// 48 kHz / 480 Hz: every substep lands exactly on a sample boundary.
pub const SAMPLES_PER_SUBSTEP: u64 = SAMPLE_RATE as u64 / SUBSTEP_HZ;

const NOTE_SECONDS: f64 = 1.0;
const NOTE_GAIN: f64 = 0.3;
const ATTACK_SECONDS: f64 = 0.004;
const DECAY_SECONDS: f64 = 0.25;

/// Renders a mono soundtrack of `duration_seconds`, one note per Boundary Hit.
pub fn synthesize(record: &SimRecord, duration_seconds: f64) -> Vec<f32> {
    let total = (duration_seconds * SAMPLE_RATE as f64).round() as usize;
    let mut mix = vec![0.0f64; total];
    let note_len = (NOTE_SECONDS * SAMPLE_RATE as f64) as usize;

    for hit in &record.boundary_hits {
        let start = (hit.substep * SAMPLES_PER_SUBSTEP) as usize;
        if start >= total {
            continue;
        }
        let end = (start + note_len).min(total);
        for (i, sample) in mix[start..end].iter_mut().enumerate() {
            let t = i as f64 / SAMPLE_RATE as f64;
            let env = (t / ATTACK_SECONDS).min(1.0) * (-t / DECAY_SECONDS).exp();
            *sample += NOTE_GAIN * env * (std::f64::consts::TAU * hit.pitch * t).sin();
        }
    }

    // Soft clip so overlapping notes can never exceed full scale.
    mix.into_iter().map(|s| s.tanh() as f32).collect()
}
