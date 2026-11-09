//! The instruments: each renders one note into a buffer, additively.

use std::f64::consts::TAU;

use super::music::Instrument;
use super::SAMPLE_RATE;
use crate::sim::rng::Rng;

const SR: f64 = SAMPLE_RATE as f64;
/// Every note fades out over this long before its end, so a Voice that is cut
/// short (stolen) or reaches its length never clicks.
const RELEASE_SECONDS: f64 = 0.006;
const ATTACK_SECONDS: f64 = 0.002;

/// How long a note rings, in seconds, if nothing cuts it short.
pub fn note_seconds(instrument: Instrument) -> f64 {
    match instrument {
        Instrument::SineBell => 1.6,
        Instrument::Pluck => 1.2,
        Instrument::Marimba => 0.9,
    }
}

/// Adds one note of `pitch` Hz at `gain` into `mix` (which is exactly the
/// note's length). `noise_seed` seeds the pluck's excitation.
pub fn render(instrument: Instrument, pitch: f64, gain: f64, noise_seed: u64, mix: &mut [f64]) {
    let mut note = vec![0.0; mix.len()];
    let out = &mut note[..];
    match instrument {
        Instrument::SineBell => sine_bell(pitch, out),
        Instrument::Pluck => pluck(pitch, noise_seed, out),
        Instrument::Marimba => marimba(pitch, out),
    }
    let attack = (ATTACK_SECONDS * SR) as usize;
    let release = ((RELEASE_SECONDS * SR) as usize).min(out.len());
    let len = out.len();
    for (i, s) in out.iter_mut().enumerate() {
        let a = if i < attack {
            i as f64 / attack as f64
        } else {
            1.0
        };
        let r = if i + release >= len {
            (len - i) as f64 / release as f64
        } else {
            1.0
        };
        *s *= gain * a * r;
    }
    for (m, s) in mix.iter_mut().zip(&note) {
        *m += s;
    }
}

/// Mostly-harmonic partials, the upper ones dying faster: a soft bell.
fn sine_bell(pitch: f64, out: &mut [f64]) {
    // (frequency ratio, level, decay time constant in seconds)
    const PARTIALS: [(f64, f64, f64); 4] = [
        (1.0, 1.0, 0.9),
        (2.0, 0.35, 0.45),
        (3.0, 0.12, 0.22),
        (4.16, 0.06, 0.12),
    ];
    for (ratio, level, tau) in PARTIALS {
        let f = pitch * ratio;
        if f >= SR * 0.45 {
            continue;
        }
        let step = TAU * f / SR;
        let decay = (-1.0 / (tau * SR)).exp();
        let mut env = level * 0.7;
        for (i, s) in out.iter_mut().enumerate() {
            *s += env * (step * i as f64).sin();
            env *= decay;
        }
    }
}

/// Karplus-Strong plucked string: a burst of filtered noise in a delay line,
/// averaged every pass so it rings at the line's period and mellows as it dies.
fn pluck(pitch: f64, noise_seed: u64, out: &mut [f64]) {
    let period = (SR / pitch).round().max(2.0) as usize;
    let mut rng = Rng::from_seed(noise_seed);
    let mut line: Vec<f64> = (0..period).map(|_| rng.range_f64(-1.0, 1.0)).collect();
    // Soften the excitation (one-pole lowpass) for a rounder, less wiry tone.
    let mut prev = 0.0;
    for s in &mut line {
        prev = 0.5 * prev + 0.5 * *s;
        *s = prev;
    }
    // Loss per sample chosen so every pitch rings for about the same time.
    let loss = (-1.0 / (0.7 * SR)).exp();
    let mut pos = 0;
    for s in out.iter_mut() {
        let next = (pos + 1) % period;
        let y = line[pos];
        *s += 1.5 * y;
        line[pos] = loss * 0.5 * (y + line[next]);
        pos = next;
    }
}

/// Two-operator FM with a fast-decaying modulation index: a woody, marimba-like
/// strike that settles into a near-sine.
fn marimba(pitch: f64, out: &mut [f64]) {
    let carrier = TAU * pitch / SR;
    let modulator = TAU * pitch * 4.0 / SR;
    let amp_decay = (-1.0 / (0.32 * SR)).exp();
    let index_decay = (-1.0 / (0.025 * SR)).exp();
    let (mut amp, mut index) = (0.8, 2.2);
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f64;
        *s += amp * (carrier * t + index * (modulator * t).sin()).sin();
        amp *= amp_decay;
        index *= index_decay;
    }
}
