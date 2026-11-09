//! End Conditions. The time cap is built into the simulation and always
//! applies; a Scenario may add one more via `[end_condition] kind = "..."`.
//!
//! Adding an End Condition: implement [`EndCondition`] (in this file or a
//! sibling module) and add a match arm in [`build`].

use serde::Deserialize;

use crate::scenario::{RuleSpec, Scenario};
use crate::sim::World;

pub const TIME_CAP: &str = "time_cap";
pub const POPULATION_CAP: &str = "population_cap";
pub const ARENA_FILLED: &str = "arena_filled";
pub const MAX_SPEED_REACHED: &str = "max_speed_reached";
pub const MIN_RADIUS_REACHED: &str = "min_radius_reached";
pub const ARENA_MIN_REACHED: &str = "arena_min_reached";

pub trait EndCondition {
    /// Id written to `SimRecord::end.condition`.
    fn id(&self) -> &'static str;

    /// Checked at the end of every substep, after Rules have run.
    fn is_met(&self, world: &World, substep: u64) -> bool;
}

/// Builds the Scenario's extra End Condition, if any. `time_cap` (or none)
/// means the time cap alone ends the Video.
pub fn build(scenario: &Scenario) -> Result<Option<Box<dyn EndCondition>>, String> {
    let Some(spec) = &scenario.end_condition else {
        return Ok(None);
    };
    match spec.kind.as_str() {
        TIME_CAP => Ok(None),
        POPULATION_CAP => {
            no_params(spec)?;
            Ok(Some(Box::new(PopulationCap {
                cap: scenario.population_cap,
            })))
        }
        ARENA_FILLED => {
            no_params(spec)?;
            Ok(Some(Box::new(ArenaFilled)))
        }
        MAX_SPEED_REACHED => {
            no_params(spec)?;
            Ok(Some(Box::new(MaxSpeedReached {
                max_speed: scenario.ball.max_speed,
            })))
        }
        MIN_RADIUS_REACHED => Ok(Some(Box::new(MinRadiusReached {
            min_radius: min_radius(spec)?,
        }))),
        ARENA_MIN_REACHED => Ok(Some(Box::new(ArenaMinReached {
            min_radius: min_radius(spec)?,
        }))),
        other => Err(format!("unknown End Condition '{other}'")),
    }
}

fn no_params(spec: &RuleSpec) -> Result<(), String> {
    match spec.params.keys().next() {
        Some(key) => Err(format!(
            "End Condition '{}' has no parameter '{key}'",
            spec.kind
        )),
        None => Ok(()),
    }
}

/// Reads the one `min_radius` parameter (finite, > 0).
fn min_radius(spec: &RuleSpec) -> Result<f64, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Params {
        min_radius: f64,
    }
    let p: Params = crate::rules::params(spec)?;
    if !(p.min_radius.is_finite() && p.min_radius > 0.0) {
        return Err(format!(
            "End Condition '{}': min_radius must be a finite number > 0",
            spec.kind
        ));
    }
    Ok(p.min_radius)
}

/// The ball count has reached the Scenario's Population Cap (which the
/// simulation never lets it exceed).
struct PopulationCap {
    cap: usize,
}

impl EndCondition for PopulationCap {
    fn id(&self) -> &'static str {
        POPULATION_CAP
    }

    fn is_met(&self, world: &World, _substep: u64) -> bool {
        world.balls.len() >= self.cap
    }
}

/// A ball has grown to fill the Arena. The simulation never lets a ball be
/// larger than the Arena, so "filled" means its radius has reached the Arena's.
struct ArenaFilled;

impl EndCondition for ArenaFilled {
    fn id(&self) -> &'static str {
        ARENA_FILLED
    }

    fn is_met(&self, world: &World, _substep: u64) -> bool {
        world
            .balls
            .iter()
            .any(|ball| ball.radius >= world.arena.radius)
    }
}

/// Speed limits are applied by scaling, so a ball at the limit can be a
/// rounding error under it.
const AT_LIMIT: f64 = 1.0 - 1e-9;

/// A ball has reached the Scenario's max speed (e.g. after speeding up on
/// every Boundary Hit).
struct MaxSpeedReached {
    max_speed: f64,
}

impl EndCondition for MaxSpeedReached {
    fn id(&self) -> &'static str {
        MAX_SPEED_REACHED
    }

    fn is_met(&self, world: &World, _substep: u64) -> bool {
        let limit = self.max_speed * AT_LIMIT;
        world.balls.iter().any(|ball| ball.vel.length() >= limit)
    }
}

/// A ball has shrunk to `min_radius` (pair with the shrink Rule's minimum).
struct MinRadiusReached {
    min_radius: f64,
}

impl EndCondition for MinRadiusReached {
    fn id(&self) -> &'static str {
        MIN_RADIUS_REACHED
    }

    fn is_met(&self, world: &World, _substep: u64) -> bool {
        world
            .balls
            .iter()
            .any(|ball| ball.radius <= self.min_radius)
    }
}

/// The Arena has shrunk to `min_radius` (pair with the shrinking_arena Rule's
/// minimum).
struct ArenaMinReached {
    min_radius: f64,
}

impl EndCondition for ArenaMinReached {
    fn id(&self) -> &'static str {
        ARENA_MIN_REACHED
    }

    fn is_met(&self, world: &World, _substep: u64) -> bool {
        world.arena.radius <= self.min_radius
    }
}
