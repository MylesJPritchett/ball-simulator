//! Rules: composable pieces of behaviour a Scenario switches on.
//!
//! Adding a Rule:
//! 1. Create `src/rules/<name>.rs` with a type implementing [`Rule`] and a
//!    `from_spec(&RuleSpec) -> Result<Self, String>` constructor (use [`params`]).
//! 2. Add one `mod` line, a kind const and one match arm in [`build`].
//!
//! Rules must be deterministic: any randomness comes from `ctx.rng`, never from
//! the clock, a thread RNG or hash-map iteration order.

mod change_colour;
mod crack;
mod drag;
mod film;
mod gap;
mod gravity;
mod grow;
mod play_note;
mod shrink;
mod shrinking_arena;
mod speed_up;
mod split;

use serde::de::DeserializeOwned;

use crate::scenario::{RuleSpec, Scenario};
use crate::sim::rng::Rng;
use crate::sim::{BoundaryHit, SimState};

/// Rule kinds, as written in Scenario files (`kind = "..."`).
pub const GRAVITY: &str = "gravity";
pub const DRAG: &str = "drag";
pub const SPEED_UP: &str = "speed_up";
pub const CHANGE_COLOUR: &str = "change_colour";
pub const SHRINKING_ARENA: &str = "shrinking_arena";
pub const SPLIT: &str = "split";
pub const GROW: &str = "grow";
pub const SHRINK: &str = "shrink";
pub const PLAY_NOTE: &str = "play_note";
pub const GAP: &str = "gap";
pub const CRACK: &str = "crack";
pub const FILM: &str = "film";

/// Per-call context handed to Rules by the simulation.
pub struct RuleContext<'a> {
    /// The simulation's single seeded RNG.
    pub rng: &'a mut Rng,
    /// Substeps completed so far (the substep being simulated is `substep + 1`).
    pub substep: u64,
    /// Substep duration in seconds.
    pub dt: f64,
    /// The Scenario's max speed (units per second).
    pub max_speed: f64,
}

/// One Rule's hooks into the simulation. All hooks default to doing nothing.
///
/// `on_start` runs once for every Rule (in Scenario order) before anything
/// else. Order within a substep: `on_substep` for every Rule (in Scenario order),
/// then integration and Boundary resolution for every ball, then
/// `on_boundary_hit` for each Boundary Hit (in ball order, Rules in Scenario order).
pub trait Rule {
    /// Whether this Rule can let a ball out of the Arena, by opening the
    /// Boundary (adding to `state.arena.gaps`) or marking a ball `escaping`.
    /// The `escaped` End Condition needs at least one such Rule.
    fn lets_balls_out(&self) -> bool {
        false
    }

    /// Whether this Rule can add balls (e.g. by splitting one). The
    /// `meltdown` End Condition needs at least one such Rule.
    fn adds_balls(&self) -> bool {
        false
    }

    /// Whether this Rule works relative to the Scenario's max speed (see
    /// `RuleContext::max_speed`), so the Scenario must set `ball.max_speed`
    /// rather than take the default.
    fn needs_max_speed(&self) -> bool {
        false
    }

    /// Called once when the simulation starts, after the first ball is
    /// placed and before the first frame (e.g. to open a Gap).
    fn on_start(&mut self, _state: &mut SimState, _ctx: &mut RuleContext) {}

    /// Called once per substep before balls move (forces, Arena changes, ...).
    fn on_substep(&mut self, _state: &mut SimState, _ctx: &mut RuleContext) {}

    /// Called for each Boundary Hit after the ball has been reflected and pushed
    /// back inside. `ball` is the ball's index in `state.balls`. Rules may append
    /// balls to `state.balls` (e.g. split) or remove one with `state.vanish`
    /// (e.g. shrink); they must not remove or reorder balls any other way.
    fn on_boundary_hit(
        &mut self,
        _state: &mut SimState,
        _ball: usize,
        _hit: &mut BoundaryHit,
        _ctx: &mut RuleContext,
    ) {
    }
}

/// Builds one Rule from its Scenario entry.
pub fn build(spec: &RuleSpec) -> Result<Box<dyn Rule>, String> {
    match spec.kind.as_str() {
        GRAVITY => Ok(Box::new(gravity::Gravity::from_spec(spec)?)),
        DRAG => Ok(Box::new(drag::Drag::from_spec(spec)?)),
        SPEED_UP => Ok(Box::new(speed_up::SpeedUp::from_spec(spec)?)),
        CHANGE_COLOUR => Ok(Box::new(change_colour::ChangeColour::from_spec(spec)?)),
        SHRINKING_ARENA => Ok(Box::new(shrinking_arena::ShrinkingArena::from_spec(spec)?)),
        SPLIT => Ok(Box::new(split::Split::from_spec(spec)?)),
        GROW => Ok(Box::new(grow::Grow::from_spec(spec)?)),
        SHRINK => Ok(Box::new(shrink::Shrink::from_spec(spec)?)),
        PLAY_NOTE => Ok(Box::new(play_note::PlayNote::from_spec(spec)?)),
        GAP => Ok(Box::new(gap::Gap::from_spec(spec)?)),
        CRACK => Ok(Box::new(crack::Crack::from_spec(spec)?)),
        FILM => Ok(Box::new(film::Film::from_spec(spec)?)),
        other => Err(format!("unknown Rule '{other}'")),
    }
}

/// Builds every Rule a Scenario lists, in order.
pub fn build_all(scenario: &Scenario) -> Result<Vec<Box<dyn Rule>>, String> {
    scenario.rules.iter().map(build).collect()
}

/// Checks a Rule or End Condition parameter is a finite number > 0.
pub fn positive_finite(spec: &RuleSpec, name: &str, value: f64) -> Result<f64, String> {
    check_param(spec, name, value, value > 0.0, " > 0")
}

/// Checks a Rule or End Condition parameter is a finite number >= 0.
pub fn non_negative_finite(spec: &RuleSpec, name: &str, value: f64) -> Result<f64, String> {
    check_param(spec, name, value, value >= 0.0, " >= 0")
}

/// Checks a Rule or End Condition parameter is a finite number.
pub fn finite(spec: &RuleSpec, name: &str, value: f64) -> Result<f64, String> {
    check_param(spec, name, value, true, "")
}

fn check_param(
    spec: &RuleSpec,
    name: &str,
    value: f64,
    in_range: bool,
    bound: &str,
) -> Result<f64, String> {
    if value.is_finite() && in_range {
        Ok(value)
    } else {
        Err(format!(
            "'{}': {name} must be a finite number{bound}, got {value}",
            spec.kind
        ))
    }
}

/// Deserializes a Rule's parameters (every key of its table except `kind`).
pub fn params<T: DeserializeOwned>(spec: &RuleSpec) -> Result<T, String> {
    serde_path_to_error::deserialize(toml::Value::Table(spec.params.clone())).map_err(|e| {
        let message = crate::scenario::with_path(&e.path().to_string(), e.inner());
        format!("Rule '{}': {message}", spec.kind)
    })
}
