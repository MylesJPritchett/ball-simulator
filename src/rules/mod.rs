//! Rules: composable pieces of behaviour a Scenario switches on.
//!
//! Adding a Rule:
//! 1. Create `src/rules/<name>.rs` with a type implementing [`Rule`] and a
//!    `from_spec(&RuleSpec) -> Result<Self, String>` constructor (use [`params`]).
//! 2. Add one `mod` line and one match arm in [`build`].
//!
//! Rules must be deterministic: any randomness comes from `ctx.rng`, never from
//! the clock, a thread RNG or hash-map iteration order.

mod split;

use serde::de::DeserializeOwned;

use crate::scenario::{RuleSpec, Scenario};
use crate::sim::rng::Rng;
use crate::sim::{BoundaryHit, World};

/// Per-call context handed to Rules by the simulation.
pub struct RuleContext<'a> {
    /// The simulation's single seeded RNG.
    pub rng: &'a mut Rng,
    /// Substeps completed so far (the substep being simulated is `substep + 1`).
    pub substep: u64,
    /// Substep duration in seconds.
    pub dt: f64,
}

/// One Rule's hooks into the simulation. All hooks default to doing nothing.
///
/// Order within a substep: `on_substep` for every Rule (in Scenario order),
/// then integration and Boundary resolution for every ball, then
/// `on_boundary_hit` for each Boundary Hit (in ball order, Rules in Scenario order).
pub trait Rule {
    /// Called once per substep before balls move (forces, Arena changes, ...).
    fn on_substep(&mut self, _world: &mut World, _ctx: &mut RuleContext) {}

    /// Called for each Boundary Hit after the ball has been reflected and pushed
    /// back inside. `ball` is the ball's index in `world.balls`. Rules may append
    /// balls to `world.balls` (e.g. split); they must not remove or reorder balls.
    fn on_boundary_hit(
        &mut self,
        _world: &mut World,
        _ball: usize,
        _hit: &BoundaryHit,
        _ctx: &mut RuleContext,
    ) {
    }
}

/// Builds one Rule from its Scenario entry.
pub fn build(spec: &RuleSpec) -> Result<Box<dyn Rule>, String> {
    match spec.kind.as_str() {
        "split" => Ok(Box::new(split::Split::from_spec(spec)?)),
        other => Err(format!("unknown Rule '{other}'")),
    }
}

/// Builds every Rule a Scenario lists, in order.
pub fn build_all(scenario: &Scenario) -> Result<Vec<Box<dyn Rule>>, String> {
    scenario.rules.iter().map(build).collect()
}

/// Deserializes a Rule's parameters (every key of its table except `kind`).
pub fn params<T: DeserializeOwned>(spec: &RuleSpec) -> Result<T, String> {
    toml::Value::Table(spec.params.clone())
        .try_into()
        .map_err(|e| format!("Rule '{}': {e}", spec.kind))
}
