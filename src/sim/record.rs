//! The simulation record: everything downstream (audio, Quality Gate, tests)
//! needs to know about a run. Add fields freely; tests only read the ones they
//! care about.

use serde::Serialize;

use super::SUBSTEP_HZ;

/// Converts a substep count to seconds since the start of the Video.
pub fn substep_to_seconds(substep: u64) -> f64 {
    substep as f64 / SUBSTEP_HZ as f64
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimRecord {
    pub seed: u64,
    /// Every Boundary Hit, in the order they happened.
    pub boundary_hits: Vec<BoundaryHit>,
    /// Ball count over time: one sample at substep 0 and one whenever it changes.
    pub ball_count: Vec<BallCountSample>,
    /// Which End Condition fired and when.
    pub end: EndEvent,
    pub stats: SimStats,
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EndEvent {
    /// End Condition id, e.g. `"time_cap"`.
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
