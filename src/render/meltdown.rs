//! The Meltdown: the Video appears to break down under its own load.
//!
//! Staged, not real (rendering is offline, ADR 0001): every glitch is drawn
//! from the Seed on its own RNG stream, so the same Seed breaks down the same
//! way. Over the Play-out the picture stutters (a glitched frame repeats for
//! longer and longer while the action carries on underneath), tears into
//! shifted bands and splits into offset colour channels, getting worse as it
//! goes. At the freeze it cuts to one frozen broken frame for the hold.

use crate::sim::rng::Rng;

use super::layout;

/// XORed into the Seed for the Meltdown's RNG stream, so it draws nothing
/// from the simulation's.
const MELTDOWN_STREAM: u64 = 0x6d65_6c74_646f_776e;
/// Extra frames each stutter holds over the one before, on average.
const STUTTER_GROWTH: f64 = 0.6;

pub struct Meltdown {
    /// The glitched frame currently repeating, with the stutter it belongs
    /// to (`None` for the broken frame).
    held: Option<(Option<usize>, Vec<u8>)>,
}

impl Meltdown {
    pub fn new() -> Self {
        Meltdown { held: None }
    }

    /// Replaces `frame` (RGBA, just drawn) with the Meltdown's frame `index`
    /// frames into the Play-out, `t` (0..1) of the way through it.
    pub fn play_out(&mut self, frame: &mut [u8], seed: u64, index: usize, t: f32) {
        let stutter = stutter_of(seed, index);
        match &self.held {
            Some((Some(held), bytes)) if *held == stutter => frame.copy_from_slice(bytes),
            _ => {
                let mut rng = Rng::from_seed(seed ^ MELTDOWN_STREAM ^ (stutter as u64 + 1));
                glitch(frame, &mut rng, 0.3 + 0.7 * t);
                self.held = Some((Some(stutter), frame.to_vec()));
            }
        }
    }

    /// Replaces `frame` (the frozen action, just drawn) with the broken
    /// frame, the same on every frame of the hold.
    pub fn broken(&mut self, frame: &mut [u8], seed: u64) {
        match &self.held {
            Some((None, bytes)) => frame.copy_from_slice(bytes),
            _ => {
                let mut rng = Rng::from_seed(seed ^ MELTDOWN_STREAM);
                glitch(frame, &mut rng, 1.0);
                dead_bands(frame, &mut rng);
                self.held = Some((None, frame.to_vec()));
            }
        }
    }
}

/// Which stutter frame `index` of the Play-out falls in. Each stutter holds
/// one frame longer than the last on average ([`STUTTER_GROWTH`]), jittered
/// by the Seed.
fn stutter_of(seed: u64, index: usize) -> usize {
    let mut rng = Rng::from_seed(seed ^ MELTDOWN_STREAM);
    let mut end = 0;
    for stutter in 0.. {
        let length = 1.0 + stutter as f64 * STUTTER_GROWTH * rng.range_f64(0.5, 1.5);
        end += length.round() as usize;
        if index < end {
            return stutter;
        }
    }
    unreachable!()
}

/// Tears the frame into horizontally shifted bands and offsets its red and
/// blue channels in opposite directions; `strength` (0..=1) sets how many
/// bands and how far everything moves.
fn glitch(frame: &mut [u8], rng: &mut Rng, strength: f32) {
    let (width, height) = (layout::WIDTH as usize, layout::HEIGHT as usize);
    let strength = strength as f64;
    let mut pick = |lo: f64, hi: f64| rng.range_f64(lo, hi);

    let bands = (2.0 + 10.0 * strength).round() as usize;
    for _ in 0..bands {
        let top = pick(0.0, height as f64) as usize;
        let rows = pick(8.0, 40.0 + 260.0 * strength) as usize;
        let shift = pick(-1.0, 1.0) * (20.0 + 300.0 * strength);
        let shift = (shift.round() as isize).rem_euclid(width as isize) as usize * 4;
        for y in top..(top + rows).min(height) {
            frame[y * width * 4..(y + 1) * width * 4].rotate_right(shift);
        }
    }

    let offset = (4.0 + 36.0 * strength * pick(0.5, 1.0)).round() as usize;
    for row in frame.chunks_exact_mut(width * 4) {
        // Red moves right, blue moves left, green stays.
        for x in (offset..width).rev() {
            row[x * 4] = row[(x - offset) * 4];
        }
        for x in 0..width - offset {
            row[x * 4 + 2] = row[(x + offset) * 4 + 2];
        }
    }
}

/// Blanks a few bands of the frame to black, as if the signal had died.
fn dead_bands(frame: &mut [u8], rng: &mut Rng) {
    let (width, height) = (layout::WIDTH as usize, layout::HEIGHT as usize);
    for _ in 0..4 {
        let top = rng.range_f64(0.0, height as f64) as usize;
        let rows = rng.range_f64(20.0, 160.0) as usize;
        let band = &mut frame[top * width * 4..(top + rows).min(height) * width * 4];
        for px in band.as_chunks_mut::<4>().0 {
            *px = [0, 0, 0, 255];
        }
    }
}
