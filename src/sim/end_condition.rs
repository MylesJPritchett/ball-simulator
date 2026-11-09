//! End Conditions. The time cap is built into the simulation and always
//! applies; a Scenario may add one more via `[end_condition] kind = "..."`.
//!
//! Adding an End Condition: implement [`EndCondition`] (in this file or a
//! sibling module) and add a match arm in [`build`].

use crate::scenario::Scenario;
use crate::sim::World;

pub const TIME_CAP: &str = "time_cap";

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
        other => Err(format!("unknown End Condition '{other}'")),
    }
}
