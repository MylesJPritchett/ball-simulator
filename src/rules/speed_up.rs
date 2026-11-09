//! Speed-up: a ball gets faster by a percentage on each Boundary Hit.
//! The simulation caps the result at the Scenario's max speed.

use serde::Deserialize;

use super::{params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{BoundaryHit, World};

pub struct SpeedUp {
    /// Velocity multiplier applied on each Boundary Hit (1 + percent / 100).
    factor: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    percent: f64,
}

impl SpeedUp {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        if !(p.percent.is_finite() && p.percent >= 0.0) {
            return Err("Rule 'speed_up': percent must be a finite number >= 0".to_string());
        }
        Ok(SpeedUp {
            factor: 1.0 + p.percent / 100.0,
        })
    }
}

impl Rule for SpeedUp {
    fn on_boundary_hit(
        &mut self,
        world: &mut World,
        ball: usize,
        _hit: &mut BoundaryHit,
        _ctx: &mut RuleContext,
    ) {
        let ball = &mut world.balls[ball];
        ball.vel = ball.vel * self.factor;
    }
}
