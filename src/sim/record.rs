//! The simulation record: everything downstream (audio, Quality Gate, tests)
//! needs to know about a run. Add fields freely; tests only read the ones they
//! care about.

use serde::Serialize;

use super::{Ball, Ending, Verdict, SUBSTEP_HZ};

/// Converts a substep count to seconds since the start of the Video.
pub fn substep_to_seconds(substep: u64) -> f64 {
    substep as f64 / SUBSTEP_HZ as f64
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimRecord {
    pub seed: u64,
    /// Each Scenario range and the value this Seed resolved it to, in draw order.
    pub parameters: Vec<crate::scenario::ResolvedParam>,
    /// Instrument, scale and pitch mode for this Video.
    pub sound: crate::audio::music::Sound,
    /// Name of the Video's palette (pinned by the Scenario or chosen by the Seed).
    pub palette: String,
    /// Every Boundary Hit, in the order they happened. The Play-out is on
    /// screen, so hits during it are here too (and play their notes), up to
    /// and including the freeze substep.
    pub boundary_hits: Vec<BoundaryHit>,
    /// Ball count over time: one sample at substep 0 and one whenever it changes.
    pub ball_count: Vec<BallCountSample>,
    /// Film left over time: one sample at substep 0 and one whenever it
    /// changes. Empty without a Film.
    pub film_left: Vec<FilmLeftSample>,
    /// Which End Condition fired and when.
    pub end: EndEvent,
    /// Timing of the Play-out, freeze, flash and hold that follow the End
    /// Condition.
    pub ending: Ending,
    pub stats: SimStats,
    /// The Quality Gate's verdict on this Seed.
    pub quality_gate: Verdict,
    /// Arena radius when the run ended (differs from the Scenario's if a Rule
    /// changed it, e.g. shrinking_arena).
    pub final_arena_radius: f64,
    /// Every ball as it was when the run ended.
    pub final_balls: Vec<Ball>,
    /// Hash of the final simulation state; changes if behaviour changes at all.
    pub final_state_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BoundaryHit {
    /// Substeps completed when the hit was resolved; time = substep / SUBSTEP_HZ.
    pub substep: u64,
    pub ball_id: u32,
    /// Angle (radians, `atan2(y, x)`, y down) of the hit point on the Boundary.
    pub angle: f64,
    /// Pitch assigned to this hit's note, in Hz.
    pub pitch: f64,
    /// The ball's speed at impact (before on-hit Rules), in units per second.
    pub speed: f64,
    /// The ball's palette colour index after on-hit Rules ran.
    pub colour: u32,
    /// Whether this hit's note sounds (set by the `play_note` Rule). The pitch
    /// is assigned either way.
    pub plays_note: bool,
}

impl BoundaryHit {
    pub fn time(&self) -> f64 {
        substep_to_seconds(self.substep)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BallCountSample {
    pub substep: u64,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FilmLeftSample {
    pub substep: u64,
    /// The fraction of the Arena that still has Film (see `Film::left`).
    pub left: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EndEvent {
    /// End Condition id, e.g. `"arena_filled"`, or `"time_cap"` if the time
    /// cap ended the run instead.
    pub condition: String,
    /// Substeps completed when it fired.
    pub substep: u64,
}

impl EndEvent {
    pub fn time(&self) -> f64 {
        substep_to_seconds(self.substep)
    }
}

/// Aggregate measurements taken every substep, for invariant tests.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct SimStats {
    /// Largest distance any ball's edge was outside the Boundary at the end of a
    /// substep (0 if never).
    pub max_boundary_overshoot: f64,
    /// Highest ball speed at the end of any substep, in units per second.
    pub max_speed: f64,
}
