//! The ending: what happens after an End Condition fires.
//!
//! The End Condition chooses its Ending's shape ([`EndingStyle`]): an optional
//! Play-out in which the simulation keeps going and the Terminal State finishes
//! on screen, whether a flash plays and the final chord sounds, and which
//! effect the renderer draws over the Play-out. After the Play-out the action
//! freezes and the frozen frame holds for [`HOLD_SECONDS`] before the Video
//! cuts. Everything downstream reads the timing from [`Ending`] (via
//! `SimRecord::ending`, built only by `Simulation::ending`) so render, audio
//! (final chord) and the Quality Gate agree.

use serde::Serialize;

use super::record::substep_to_seconds;
use super::{FPS, SUBSTEPS_PER_FRAME, SUBSTEP_HZ};

/// How long the frozen final frame holds before the cut.
pub const HOLD_SECONDS: f64 = 1.5;
/// How long the flash takes to fade from full to nothing.
pub const FLASH_SECONDS: f64 = 0.4;

fn to_substeps(seconds: f64) -> u64 {
    (seconds * SUBSTEP_HZ as f64).round() as u64
}

/// The shape of an End Condition's Ending (see `EndCondition::ending`). The
/// default is no Play-out, with a flash and the final chord.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndingStyle {
    pub play_out: PlayOut,
    /// Whether the flash plays when the End Condition fires.
    pub flash: bool,
    /// Whether the final chord sounds when the End Condition fires.
    pub final_chord: bool,
    /// What the renderer draws over the Play-out.
    pub effect: Effect,
}

impl Default for EndingStyle {
    fn default() -> Self {
        EndingStyle {
            play_out: PlayOut::None,
            flash: true,
            final_chord: true,
            effect: Effect::None,
        }
    }
}

/// How long the simulation keeps going after the End Condition fires.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayOut {
    /// The action freezes on the substep the End Condition fired.
    None,
    /// The action freezes `seconds` after the End Condition fired.
    For { seconds: f64 },
    /// The action freezes once `EndCondition::play_out_done` is true (checked
    /// every substep from the one the End Condition fired on), or
    /// `max_seconds` after it fired, whichever comes first.
    Until { max_seconds: f64 },
}

impl PlayOut {
    /// The longest the Play-out can last, in substeps.
    pub fn max_substeps(&self) -> u64 {
        match *self {
            PlayOut::None => 0,
            PlayOut::For { seconds } => to_substeps(seconds),
            PlayOut::Until { max_seconds } => to_substeps(max_seconds),
        }
    }
}

/// What the renderer draws over the Play-out, timed by [`Ending::play_out_at`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    None,
    /// An expanding, fading ring where each vanished ball was.
    Pop,
    /// The Video appears to break down: frames stutter, tear and split into
    /// colour channels, then cut to a frozen broken frame for the hold.
    Meltdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Ending {
    /// Substep the End Condition fired; the Play-out, the flash and the final
    /// chord start here.
    pub start_substep: u64,
    /// Substep the action freezes (the end of the Play-out; `start_substep`
    /// without one).
    pub freeze_substep: u64,
    /// Substeps the flash takes to fade out (0: no flash).
    pub flash_substeps: u64,
    /// Substep the Video cuts (end of the hold).
    pub cut_substep: u64,
    /// Whether the final chord sounds at `start_substep`.
    pub final_chord: bool,
    /// What the renderer draws over the Play-out.
    pub effect: Effect,
}

impl Ending {
    pub fn new(start_substep: u64, freeze_substep: u64, style: &EndingStyle) -> Self {
        Ending {
            start_substep,
            freeze_substep,
            flash_substeps: if style.flash {
                to_substeps(FLASH_SECONDS)
            } else {
                0
            },
            cut_substep: freeze_substep + to_substeps(HOLD_SECONDS),
            final_chord: style.final_chord,
            effect: style.effect,
        }
    }

    pub fn start_time(&self) -> f64 {
        substep_to_seconds(self.start_substep)
    }

    pub fn cut_time(&self) -> f64 {
        substep_to_seconds(self.cut_substep)
    }

    /// Video frames, including the Play-out and the hold. Frame k shows the
    /// moment after `k * SUBSTEPS_PER_FRAME` substeps (frozen once past
    /// `freeze_substep`).
    pub fn frames(&self) -> u64 {
        self.cut_substep.div_ceil(SUBSTEPS_PER_FRAME)
    }

    /// Video duration in seconds (whole frames).
    pub fn duration(&self) -> f64 {
        self.frames() as f64 / FPS as f64
    }

    /// Flash strength (0 = none, 1 = full) at a moment, in substeps from the
    /// start of the Video. Full when the ending starts, fading linearly to 0.
    pub fn flash_at(&self, substep: u64) -> f32 {
        if substep < self.start_substep || self.flash_substeps == 0 {
            return 0.0;
        }
        let since = (substep - self.start_substep) as f32;
        (1.0 - since / self.flash_substeps as f32).max(0.0)
    }

    /// How far through the Play-out a moment is (0 when the End Condition
    /// fires, approaching 1 at the freeze), or `None` outside the Play-out.
    pub fn play_out_at(&self, substep: u64) -> Option<f32> {
        (self.start_substep..self.freeze_substep)
            .contains(&substep)
            .then(|| {
                (substep - self.start_substep) as f32
                    / (self.freeze_substep - self.start_substep) as f32
            })
    }
}
