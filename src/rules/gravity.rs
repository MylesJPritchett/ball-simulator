//! Gravity: constant downward acceleration on every ball.

use serde::Deserialize;

use super::{finite, params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{SimState, Vec2};

pub struct Gravity {
    /// Downward acceleration in units per second squared (y points down).
    strength: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    strength: f64,
}

impl Gravity {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        Ok(Gravity {
            strength: finite(spec, "strength", p.strength)?,
        })
    }
}

impl Rule for Gravity {
    fn on_substep(&mut self, state: &mut SimState, ctx: &mut RuleContext) {
        let dv = Vec2::new(0.0, self.strength * ctx.dt);
        for ball in &mut state.balls {
            ball.vel += dv;
        }
    }
}
