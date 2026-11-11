//! The Counter: the live number shown below the Arena.

use serde::Deserialize;

use crate::sim::Simulation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Counter {
    /// Boundary Hits so far.
    #[default]
    BoundaryHits,
    /// Balls currently in the Arena.
    BallCount,
    /// Speed of the fastest ball, in simulation units per second (rounded).
    Speed,
    /// Rings settled so far (see the `settle` Rule).
    RingsSettled,
}

impl Counter {
    /// The Counter's value for the simulation's current state.
    pub fn read(self, sim: &Simulation) -> u64 {
        match self {
            Counter::BoundaryHits => sim.boundary_hits().len() as u64,
            Counter::BallCount => sim.state().balls.len() as u64,
            Counter::Speed => sim
                .state()
                .balls
                .iter()
                .map(|b| b.vel.length())
                .fold(0.0, f64::max)
                .round() as u64,
            Counter::RingsSettled => sim.state().settles.len() as u64,
        }
    }

    /// Caption shown under the number.
    pub fn label(self, value: u64) -> &'static str {
        match (self, value) {
            (Counter::BoundaryHits, 1) => "BOUNCE",
            (Counter::BoundaryHits, _) => "BOUNCES",
            (Counter::BallCount, 1) => "BALL",
            (Counter::BallCount, _) => "BALLS",
            (Counter::Speed, _) => "SPEED",
            (Counter::RingsSettled, 1) => "RING",
            (Counter::RingsSettled, _) => "RINGS",
        }
    }
}
