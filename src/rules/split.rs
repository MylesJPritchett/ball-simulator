//! Split: on a Boundary Hit, a ball becomes `into` balls fanned symmetrically
//! around its reflected direction across `spread` degrees.
//!
//! ```toml
//! [[rules]]
//! kind = "split"
//! into = 2             # balls after the split (>= 2)
//! spread = 30.0        # total fan angle in degrees, 0 < spread < 180
//! speed_jitter = 0.1   # optional: each ball's speed varies up to ±10% (< 1)
//! angle_jitter = 2.0   # optional: each ball turns up to ±2 degrees off its slot
//! ```
//!
//! The ball itself takes the first direction of the fan; the rest are new
//! balls with the same position, speed, radius and colour. Jitter (none by
//! default) keeps a swarm's balls from sharing one energy and moving in step.
//! The Population Cap is enforced by the simulation, not here.

use serde::Deserialize;

use super::{non_negative_finite, params, Rule, RuleContext};
use crate::scenario::RuleSpec;
use crate::sim::{BoundaryHit, SimState};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Params {
    into: u32,
    spread: f64,
    #[serde(default)]
    speed_jitter: f64,
    #[serde(default)]
    angle_jitter: f64,
}

pub struct Split {
    into: u32,
    spread: f64,
    speed_jitter: f64,
    /// Radians.
    angle_jitter: f64,
}

impl Split {
    pub fn from_spec(spec: &RuleSpec) -> Result<Self, String> {
        let p: Params = params(spec)?;
        if p.into < 2 {
            return Err("Rule 'split': into must be at least 2".into());
        }
        // Keeping the fan under 180 degrees keeps every child moving inward.
        if !(p.spread > 0.0 && p.spread < 180.0) {
            return Err("Rule 'split': spread must be between 0 and 180 degrees".into());
        }
        let speed_jitter = non_negative_finite(spec, "speed_jitter", p.speed_jitter)?;
        // Below 1 so no ball can come out of a split stopped or reversed.
        if speed_jitter >= 1.0 {
            return Err("Rule 'split': speed_jitter must be less than 1".into());
        }
        Ok(Split {
            into: p.into,
            spread: p.spread.to_radians(),
            speed_jitter,
            angle_jitter: non_negative_finite(spec, "angle_jitter", p.angle_jitter)?.to_radians(),
        })
    }
}

impl Rule for Split {
    fn on_boundary_hit(
        &mut self,
        state: &mut SimState,
        ball: usize,
        _hit: &mut BoundaryHit,
        ctx: &mut RuleContext,
    ) {
        let parent = state.balls[ball].clone();
        let step = self.spread / (self.into - 1) as f64;
        let mut vel = |i: u32| {
            let mut turn = -self.spread / 2.0 + step * i as f64;
            let mut scale = 1.0;
            // No draws without jitter, so jitter-free Scenarios keep their runs.
            if self.angle_jitter > 0.0 {
                turn += ctx.rng.range_f64(-self.angle_jitter, self.angle_jitter);
            }
            if self.speed_jitter > 0.0 {
                scale += ctx.rng.range_f64(-self.speed_jitter, self.speed_jitter);
            }
            parent.vel.rotated(turn) * scale
        };
        state.balls[ball].vel = vel(0);
        for i in 1..self.into {
            let child = state.spawn(parent.pos, vel(i), parent.radius);
            state.balls[child].colour = parent.colour;
        }
    }
}
