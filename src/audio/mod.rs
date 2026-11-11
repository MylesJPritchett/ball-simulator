//! Audio: synthesizes the whole soundtrack offline from the simulation record,
//! sample-accurately from Boundary Hit substep timestamps, and writes it as WAV.
//!
//! The soundtrack is a pure function of the record: pitches were assigned by
//! the simulation (see [`music`]), and this module only schedules Voices,
//! renders them with the record's instrument, adds the final chord, then
//! applies a light reverb and a limiter. A Meltdown crackles, stutters and
//! cuts to silence instead of ending on the chord (see [`meltdown`]).

mod meltdown;
mod mix;
pub mod music;
mod synth;
mod wav;

pub use wav::write_wav;

use crate::sim::{Finale, SimRecord, SUBSTEP_HZ};

pub const SAMPLE_RATE: u32 = 48_000;
/// 48 kHz / 480 Hz: every substep lands exactly on a sample boundary.
pub const SAMPLES_PER_SUBSTEP: u64 = SAMPLE_RATE as u64 / SUBSTEP_HZ;

/// Most Voices sounding at once. A new note beyond this steals a sounding
/// Voice (the same pitch if one is ringing, else the oldest).
pub const MAX_VOICES: usize = 16;

/// Level of one Boundary Hit's note.
const HIT_GAIN: f64 = 0.16;
/// Level of each final chord note.
const CHORD_GAIN: f64 = 0.22;
/// Level of each note of the quiet final chord a settle plays: about a third
/// of the full chord's.
const SETTLE_CHORD_GAIN: f64 = CHORD_GAIN / 3.0;
/// The final chord rings for this long (capped at the end of the Video).
const CHORD_SECONDS: f64 = 3.0;

/// One scheduled note: sounds on samples `start..end`.
#[derive(Debug, Clone, PartialEq)]
pub struct Voice {
    pub start: usize,
    pub end: usize,
    pub pitch: f64,
    pub gain: f64,
}

/// Total soundtrack length in samples: the Video's duration, ending included.
fn total_samples(record: &SimRecord) -> usize {
    (record.ending.duration() * SAMPLE_RATE as f64).round() as usize
}

/// Schedules every note of the soundtrack: one per distinct pitch per substep
/// among the Boundary Hits that play a note
/// (simultaneous hits on the same note merge into one, a little louder), then
/// a quiet final chord at each settle and the full final chord at the Ending's
/// accent, all under [`MAX_VOICES`].
pub fn voices(record: &SimRecord) -> Vec<Voice> {
    let total = total_samples(record);
    let note_len = (synth::note_seconds(record.sound.instrument) * SAMPLE_RATE as f64) as usize;

    // (start sample, pitch, hits merged), in time order.
    let mut notes: Vec<(usize, f64, usize)> = Vec::new();
    for hit in record.boundary_hits.iter().filter(|h| h.plays_note) {
        let start = (hit.substep * SAMPLES_PER_SUBSTEP) as usize;
        match notes
            .iter_mut()
            .rev()
            .take_while(|n| n.0 == start)
            .find(|n| n.1 == hit.pitch)
        {
            Some(note) => note.2 += 1,
            None => notes.push((start, hit.pitch, 1)),
        }
    }

    let mut out: Vec<Voice> = Vec::new();
    // Indices into `out` of Voices that may still be sounding, oldest first.
    let mut sounding: Vec<usize> = Vec::new();
    let mut play = |out: &mut Vec<Voice>, start: usize, len: usize, pitch: f64, gain: f64| {
        if start >= total {
            return;
        }
        sounding.retain(|&i| out[i].end > start);
        if sounding.len() >= MAX_VOICES {
            let steal = sounding
                .iter()
                .position(|&i| out[i].pitch == pitch)
                .unwrap_or(0);
            out[sounding.remove(steal)].end = start;
        }
        sounding.push(out.len());
        out.push(Voice {
            start,
            end: (start + len).min(total),
            pitch,
            gain,
        });
    };

    // (start sample, length, pitch, gain) of every note, chords included.
    let mut scheduled: Vec<(usize, usize, f64, f64)> = notes
        .iter()
        .map(|&(start, pitch, count)| {
            let gain = HIT_GAIN * (1.0 + 0.25 * (count as f64).ln());
            (start, note_len, pitch, gain)
        })
        .collect();

    let sound = &record.sound;
    let chord_len = (CHORD_SECONDS * SAMPLE_RATE as f64) as usize;
    let mut chord = |substep: u64, gain: f64| {
        let start = (substep * SAMPLES_PER_SUBSTEP) as usize;
        for &semis in sound.scale.chord() {
            scheduled.push((start, chord_len, sound.hz(semis), gain));
        }
    };
    // The full chord sounds at the Ending's accent: when the End Condition
    // fires or at the freeze, as the End Condition chooses.
    let ending = &record.ending;
    let full_chord = ending.final_chord.then_some(ending.accent_substep);
    // Each settle plays the chord quietly, except one the full chord already
    // marks (the last, when it makes the Arena solid), so it sounds once.
    for settle in &record.settles {
        if Some(settle.substep) != full_chord {
            chord(settle.substep, SETTLE_CHORD_GAIN);
        }
    }
    if let Some(substep) = full_chord {
        chord(substep, CHORD_GAIN);
    }

    // In time order, so the Voice cap steals the right notes.
    scheduled.sort_by_key(|n| n.0);
    for (start, len, pitch, gain) in scheduled {
        play(&mut out, start, len, pitch, gain);
    }

    // Notes stolen the instant they started never sound.
    out.retain(|v| v.end > v.start);
    out
}

/// Renders the mono soundtrack for the whole Video (through the ending's hold).
pub fn synthesize(record: &SimRecord) -> Vec<f32> {
    let total = total_samples(record);
    let mut dry = vec![0.0f64; total];
    for (i, voice) in voices(record).iter().enumerate() {
        synth::render(
            record.sound.instrument,
            voice.pitch,
            voice.gain,
            record.seed ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
            &mut dry[voice.start..voice.end],
        );
    }
    let ending = &record.ending;
    let meltdown = ending.finale == Finale::Meltdown;
    if meltdown {
        meltdown::crackle(&mut dry, ending, record.seed);
    }
    let mut out = mix::limit(&mix::reverb(&dry));
    if meltdown {
        meltdown::stutter_and_cut(&mut out, ending, record.seed);
    }
    out
}
