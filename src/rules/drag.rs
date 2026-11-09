//! Drag: every ball's speed decays exponentially over time.

use serde::Deserialize;

use super::{non_negative_finite, params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::SimState;

pub struct Drag {
    /// Decay rate per second: speed follows `v0 * e^(-strength * t)`.
    strength: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    strength: f64,
}

impl Drag {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        Ok(Drag {
            strength: non_negative_finite(spec, "strength", p.strength)?,
        })
    }
}

impl Rule for Drag {
    fn on_substep(&mut self, state: &mut SimState, ctx: &mut RuleContext) {
        let factor = (-self.strength * ctx.dt).exp();
        for ball in &mut state.balls {
            ball.vel = ball.vel * factor;
        }
    }
}
