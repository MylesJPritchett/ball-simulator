//! Gravity: constant downward acceleration on every ball.

use serde::Deserialize;

use super::{params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{Vec2, World};

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
        if !p.strength.is_finite() {
            return Err("Rule 'gravity': strength must be a finite number".to_string());
        }
        Ok(Gravity {
            strength: p.strength,
        })
    }
}

impl Rule for Gravity {
    fn on_substep(&mut self, world: &mut World, ctx: &mut RuleContext) {
        let dv = Vec2::new(0.0, self.strength * ctx.dt);
        for ball in &mut world.balls {
            ball.vel += dv;
        }
    }
}
