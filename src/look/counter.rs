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
    /// Film left, as a percentage rounded up: never 0 while any Film is left.
    /// 0 in a Scenario without a `film` Rule.
    FilmLeft,
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
            // The tolerance keeps float noise (0.37 * 100 = 37.000000000000004)
            // from rounding up a whole percent; a single Film cell is far
            // more than it.
            Counter::FilmLeft => sim.state().film.as_ref().map_or(0, |film| {
                (film.left() * 100.0 - 1e-9).ceil().max(0.0) as u64
            }),
        }
    }

    /// The Counter's value as drawn: the number, with its unit if it has one.
    pub fn text(self, sim: &Simulation) -> String {
        let value = self.read(sim);
        match self {
            Counter::FilmLeft => format!("{value}%"),
            _ => value.to_string(),
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
            (Counter::FilmLeft, _) => "FILM LEFT",
        }
    }
}
