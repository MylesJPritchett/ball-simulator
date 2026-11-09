//! The musical choices for a Video: instrument, scale and pitch mode, and how a
//! Boundary Hit's pitch is picked. The simulation calls [`Sound::pitch`] for
//! every hit, so pitches land in the record and the soundtrack stays a pure
//! function of the record.
//!
//! Scenario files may pin any of these in an optional `[sound]` table:
//!
//! ```toml
//! [sound]
//! instrument = "pluck"          # "sine_bell" | "pluck" | "marimba"; omitted = Seed chooses
//! scale = "dorian"              # "major_pentatonic" | "minor_pentatonic" | "lydian" | "dorian"
//!                               # | "mixolydian" | "aeolian" | "suspended_pentatonic" | "hirajoshi";
//!                               # omitted = Seed chooses
//! pitch_mode = "angle"          # "step" (default) | "ball_size" | "angle"
//! ```

use serde::{Deserialize, Serialize};

use crate::sim::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Instrument {
    SineBell,
    Pluck,
    Marimba,
}

impl Instrument {
    pub const ALL: [Instrument; 3] = [Instrument::SineBell, Instrument::Pluck, Instrument::Marimba];

    pub fn name(self) -> &'static str {
        match self {
            Instrument::SineBell => "sine_bell",
            Instrument::Pluck => "pluck",
            Instrument::Marimba => "marimba",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    MajorPentatonic,
    MinorPentatonic,
    Lydian,
    Dorian,
    Mixolydian,
    Aeolian,
    SuspendedPentatonic,
    Hirajoshi,
}

impl Scale {
    pub const ALL: [Scale; 8] = [
        Scale::MajorPentatonic,
        Scale::MinorPentatonic,
        Scale::Lydian,
        Scale::Dorian,
        Scale::Mixolydian,
        Scale::Aeolian,
        Scale::SuspendedPentatonic,
        Scale::Hirajoshi,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Scale::MajorPentatonic => "major_pentatonic",
            Scale::MinorPentatonic => "minor_pentatonic",
            Scale::Lydian => "lydian",
            Scale::Dorian => "dorian",
            Scale::Mixolydian => "mixolydian",
            Scale::Aeolian => "aeolian",
            Scale::SuspendedPentatonic => "suspended_pentatonic",
            Scale::Hirajoshi => "hirajoshi",
        }
    }

    /// Semitones above the root of each degree in one octave.
    pub fn intervals(self) -> &'static [i32] {
        match self {
            Scale::MajorPentatonic => &[0, 2, 4, 7, 9],
            Scale::MinorPentatonic => &[0, 3, 5, 7, 10],
            Scale::Lydian => &[0, 2, 4, 6, 7, 9, 11],
            Scale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Scale::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            Scale::Aeolian => &[0, 2, 3, 5, 7, 8, 10],
            Scale::SuspendedPentatonic => &[0, 2, 5, 7, 10],
            Scale::Hirajoshi => &[0, 2, 3, 7, 8],
        }
    }

    /// The final chord, in semitones above the root (all in the scale).
    pub fn chord(self) -> &'static [i32] {
        match self {
            Scale::MajorPentatonic | Scale::Lydian | Scale::Mixolydian => &[-12, 0, 4, 7, 12],
            Scale::MinorPentatonic | Scale::Aeolian | Scale::Hirajoshi => &[-12, 0, 3, 7, 12],
            Scale::SuspendedPentatonic => &[-12, 0, 7, 12],
            Scale::Dorian => &[-12, 0, 3, 7, 10],
        }
    }

    /// Semitones above the root of `degree` (0-based, may exceed one octave).
    pub fn semitones(self, degree: usize) -> i32 {
        let iv = self.intervals();
        12 * (degree / iv.len()) as i32 + iv[degree % iv.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PitchMode {
    /// A global step counter: each Boundary Hit is the next degree up, turning
    /// around at the top and bottom of the range.
    #[default]
    Step,
    /// Bigger balls play lower notes.
    BallSize,
    /// The hit's angle around the Boundary picks the degree.
    Angle,
}

impl PitchMode {
    pub fn name(self) -> &'static str {
        match self {
            PitchMode::Step => "step",
            PitchMode::BallSize => "ball_size",
            PitchMode::Angle => "angle",
        }
    }
}

/// The optional `[sound]` table: pins; anything omitted is chosen by the Seed
/// (pitch mode defaults to `step`).
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SoundSpec {
    pub instrument: Option<Instrument>,
    pub scale: Option<Scale>,
    pub pitch_mode: PitchMode,
}

/// Octaves the pitch modes span above the root.
pub const OCTAVES: usize = 2;
/// Root note of every scale: C4.
pub const ROOT_HZ: f64 = 261.625_565_300_598_6;

/// Its own random stream, so choosing the sound never draws from the
/// simulation RNG (and never shifts other per-Seed choices).
const SOUND_STREAM: u64 = 0x50_55_4e_44_5f_8a_3c_e1;

/// A Video's resolved sound.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Sound {
    pub instrument: Instrument,
    pub scale: Scale,
    pub pitch_mode: PitchMode,
    pub root_hz: f64,
}

impl Sound {
    /// Pinned values win; the rest come from the Seed.
    pub fn for_video(spec: &SoundSpec, seed: u64) -> Self {
        let mut rng = Rng::from_seed(seed ^ SOUND_STREAM);
        let instrument = Instrument::ALL[(rng.next_u64() % Instrument::ALL.len() as u64) as usize];
        let scale = Scale::ALL[(rng.next_u64() % Scale::ALL.len() as u64) as usize];
        Sound {
            instrument: spec.instrument.unwrap_or(instrument),
            scale: spec.scale.unwrap_or(scale),
            pitch_mode: spec.pitch_mode,
            root_hz: ROOT_HZ,
        }
    }

    /// Number of distinct degrees the pitch modes use.
    pub fn degrees(&self) -> usize {
        self.scale.intervals().len() * OCTAVES
    }

    /// Frequency of `semitones` above the root.
    pub fn hz(&self, semitones: i32) -> f64 {
        self.root_hz * 2f64.powf(semitones as f64 / 12.0)
    }

    /// Pitch for a Boundary Hit: `hit_index` is the number of hits before it,
    /// `angle` its angle on the Boundary, `size` the ball radius as a fraction
    /// of the Arena radius.
    pub fn pitch(&self, hit_index: usize, angle: f64, size: f64) -> f64 {
        let n = self.degrees();
        let degree = match self.pitch_mode {
            PitchMode::Step => {
                // Up to the top and back down; neither turning note repeats.
                let period = (2 * (n - 1)).max(1);
                let phase = hit_index % period;
                if phase < n {
                    phase
                } else {
                    period - phase
                }
            }
            PitchMode::Angle => {
                let turn = (angle + std::f64::consts::PI) / std::f64::consts::TAU;
                ((turn * n as f64) as usize).min(n - 1)
            }
            PitchMode::BallSize => {
                // Log scale from 1% of the Arena (highest) to filling it (lowest).
                const SMALLEST: f64 = 0.01;
                let t = ((size / SMALLEST).log2() / (1.0 / SMALLEST).log2()).clamp(0.0, 1.0);
                ((1.0 - t) * (n - 1) as f64).round() as usize
            }
        };
        self.hz(self.scale.semitones(degree))
    }
}
