//! Drag: every ball's speed decays exponentially over time.

use serde::Deserialize;

use super::{params, Rule, RuleContext};
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
        if !(p.strength.is_finite() && p.strength >= 0.0) {
            return Err("Rule 'drag': strength must be a finite number >= 0".to_string());
        }
        Ok(Drag {
            strength: p.strength,
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
