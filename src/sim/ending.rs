//! The ending: what happens after an End Condition fires.
//!
//! The action freezes on the state at the End Condition's substep, a flash plays
//! and fades, and the frozen frame holds for [`HOLD_SECONDS`] before the Video
//! cuts. Everything downstream reads the timing from here (via
//! `SimRecord::ending`) so render, audio (final chord) and the Quality Gate agree.

use serde::Serialize;

use super::record::substep_to_seconds;
use super::{FPS, SUBSTEPS_PER_FRAME, SUBSTEP_HZ};

/// How long the frozen final frame holds before the cut.
pub const HOLD_SECONDS: f64 = 1.5;
/// How long the flash takes to fade from full to nothing.
pub const FLASH_SECONDS: f64 = 0.4;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Ending {
    /// Substep the End Condition fired; the action freezes here and the flash
    /// (and later the final chord) starts.
    pub start_substep: u64,
    /// Substeps the flash takes to fade out.
    pub flash_substeps: u64,
    /// Substep the Video cuts (end of the hold).
    pub cut_substep: u64,
}

impl Ending {
    pub fn new(start_substep: u64) -> Self {
        let to_substeps = |s: f64| (s * SUBSTEP_HZ as f64).round() as u64;
        Ending {
            start_substep,
            flash_substeps: to_substeps(FLASH_SECONDS),
            cut_substep: start_substep + to_substeps(HOLD_SECONDS),
        }
    }

    pub fn start_time(&self) -> f64 {
        substep_to_seconds(self.start_substep)
    }

    pub fn cut_time(&self) -> f64 {
        substep_to_seconds(self.cut_substep)
    }

    /// Video frames, including the hold. Frame k shows the moment after
    /// `k * SUBSTEPS_PER_FRAME` substeps (frozen once past `start_substep`).
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
}
