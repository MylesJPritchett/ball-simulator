//! Shrinking Arena: the Arena's radius closes in at a constant rate until it
//! reaches a minimum.
//!
//! The Arena shrinks in `on_substep`, before balls move, so the same substep's
//! Boundary resolution keeps every ball inside the new radius; a ball caught by
//! the closing Boundary registers a Boundary Hit like any other. The
//! simulation reflects balls off the Boundary in its own (closing) frame, so a
//! hit ball always leaves faster than the Boundary follows it.

use serde::Deserialize;

use super::{non_negative_finite, params, positive_finite, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::SimState;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    /// Arena radius lost per second, in simulation units.
    rate: f64,
    /// The Arena stops shrinking at this radius.
    min_radius: f64,
}

pub struct ShrinkingArena {
    rate: f64,
    min_radius: f64,
}

impl ShrinkingArena {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        Ok(ShrinkingArena {
            rate: non_negative_finite(spec, "rate", p.rate)?,
            min_radius: positive_finite(spec, "min_radius", p.min_radius)?,
        })
    }
}

impl Rule for ShrinkingArena {
    fn on_substep(&mut self, state: &mut SimState, ctx: &mut RuleContext) {
        let arena = &mut state.arena;
        if arena.radius > self.min_radius {
            arena.radius = (arena.radius - self.rate * ctx.dt).max(self.min_radius);
        }
    }
}
