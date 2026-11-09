//! End Conditions. The time cap is built into the simulation and always
//! applies; a Scenario may add one more via `[end_condition] kind = "..."`.
//!
//! Adding an End Condition: implement [`EndCondition`] (in this file or a
//! sibling module) and add a match arm in [`build`].

use crate::scenario::{RuleSpec, Scenario};
use crate::sim::World;

pub const TIME_CAP: &str = "time_cap";
pub const POPULATION_CAP: &str = "population_cap";
pub const ARENA_FILLED: &str = "arena_filled";

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
